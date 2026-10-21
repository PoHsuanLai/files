//! Disks: the seam to whatever lists, opens and ejects them, and the thread that serves the
//! window's asks of it.
//!
//! The window never names a bus. The binary gives a [`Volumes`] (the desktop crate's disk service
//! behind a thin adapter); a test gives [`FakeVolumes`]. One thread lists the disks at the start,
//! lists them again whenever the seam says they may have changed, and carries out mounts and
//! ejects in order, so an eject and the list after it never cross.

use std::any::Any;
use std::fmt;
use std::sync::Arc;

use files_core::{EjectFailure, FolderPath, MountFailure, Volume, VolumeId};
use files_model::{Input, TabId};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::Heard;
use crate::error::VolumeError;

/// Called by a [`Volumes`] whenever the disks may have changed.
pub type Changed = Box<dyn Fn() + Send + Sync>;

/// What keeps a watch going: dropping it stops the watch.
pub struct WatchGuard {
    /// Held only to be dropped with the guard.
    _held: Box<dyn Any + Send>,
}

impl WatchGuard {
    /// A guard that keeps `held` alive until it is dropped.
    pub fn new(held: impl Any + Send) -> WatchGuard {
        WatchGuard {
            _held: Box::new(held),
        }
    }
}

impl fmt::Debug for WatchGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("WatchGuard")
    }
}

/// The disks of the computer. Every call may block, so the edge makes them on a thread of its
/// own, never the window's.
pub trait Volumes: Send + Sync {
    /// The disks now.
    fn list(&self) -> Result<Vec<Volume>, VolumeError>;

    /// Opens (mounts) the disk, giving the folder it is at.
    fn mount(&self, id: &VolumeId) -> Result<FolderPath, MountFailure>;

    /// Ejects the disk: unmounts it and, when it can be, powers it off.
    fn eject(&self, id: &VolumeId) -> Result<(), EjectFailure>;

    /// Calls `changed` whenever a disk came, went, was opened or closed, until the guard drops.
    fn watch(&self, changed: Changed) -> Result<WatchGuard, VolumeError>;
}

/// A computer with no disks to list: the window shows none.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoVolumes;

impl Volumes for NoVolumes {
    fn list(&self) -> Result<Vec<Volume>, VolumeError> {
        Err(VolumeError::Unavailable)
    }

    fn mount(&self, _id: &VolumeId) -> Result<FolderPath, MountFailure> {
        Err(MountFailure::Gone)
    }

    fn eject(&self, _id: &VolumeId) -> Result<(), EjectFailure> {
        Err(EjectFailure::Gone)
    }

    fn watch(&self, _changed: Changed) -> Result<WatchGuard, VolumeError> {
        Err(VolumeError::Unavailable)
    }
}

/// What the disk thread is asked.
enum Ask {
    Refresh,
    Mount(TabId, VolumeId),
    Eject(TabId, VolumeId),
    Stop,
}

/// The disk thread's door. Dropping it stops the thread and the watch.
pub(super) struct Disks {
    asks: UnboundedSender<Ask>,
}

impl Disks {
    /// Starts the thread over `volumes`. Without a thread (the system would not start one) the
    /// window simply shows no disks.
    pub(super) fn start(volumes: Arc<dyn Volumes>, heard: UnboundedSender<Heard>) -> Disks {
        let (asks, mut inbox) = unbounded_channel::<Ask>();
        let refresh = asks.clone();
        let _ = std::thread::Builder::new()
            .name("files-volumes".to_owned())
            .spawn(move || {
                let tell = |heard_of: Heard| {
                    // A closed channel means the window is gone.
                    let _ = heard.send(heard_of);
                };
                let relist = || {
                    if let Ok(list) = volumes.list() {
                        tell(Heard::All(Input::Volumes(list)));
                    }
                };
                let watch = volumes.watch(Box::new(move || {
                    let _ = refresh.send(Ask::Refresh);
                }));
                relist();
                while let Some(ask) = inbox.blocking_recv() {
                    match ask {
                        Ask::Refresh => relist(),
                        Ask::Mount(tab, id) => {
                            let result = volumes.mount(&id);
                            tell(Heard::Tab(tab, Input::Mounted(id, result)));
                            relist();
                        }
                        Ask::Eject(tab, id) => {
                            let result = volumes.eject(&id);
                            tell(Heard::Tab(tab, Input::Ejected(id, result)));
                            relist();
                        }
                        Ask::Stop => break,
                    }
                }
                drop(watch);
            });
        Disks { asks }
    }

    /// Opens the disk and tells the browser where.
    pub(super) fn mount(&self, tab: TabId, id: VolumeId) {
        let _ = self.asks.send(Ask::Mount(tab, id));
    }

    /// Ejects the disk and tells the browser how it went.
    pub(super) fn eject(&self, tab: TabId, id: VolumeId) {
        let _ = self.asks.send(Ask::Eject(tab, id));
    }
}

impl Drop for Disks {
    fn drop(&mut self) {
        let _ = self.asks.send(Ask::Stop);
    }
}
