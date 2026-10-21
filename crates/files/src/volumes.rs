//! The disks, as the window sees them: the desktop crate's UDisks2 service on the runtime the
//! rest of the bus work runs on. The window's disk thread calls these and may block.

use files_core::{EjectFailure, FolderPath, MountFailure, Volume, VolumeId};
use files_desktop::Disks;
use files_ui::{Changed, VolumeError, Volumes, WatchGuard};
use tokio::runtime::Handle;
use tokio::task::JoinHandle;

/// UDisks2 behind [`Volumes`].
#[derive(Debug, Clone)]
pub struct DesktopVolumes {
    disks: Disks,
    runtime: Handle,
}

impl DesktopVolumes {
    /// The disks over `disks`, called from threads that are not on `runtime`.
    pub fn new(disks: Disks, runtime: Handle) -> DesktopVolumes {
        DesktopVolumes { disks, runtime }
    }
}

/// A task that is stopped when this is dropped.
struct Stops(JoinHandle<()>);

impl Drop for Stops {
    fn drop(&mut self) {
        self.0.abort();
    }
}

impl Volumes for DesktopVolumes {
    fn list(&self) -> Result<Vec<Volume>, VolumeError> {
        self.runtime
            .block_on(self.disks.list())
            .map_err(|_| VolumeError::Unavailable)
    }

    fn mount(&self, id: &VolumeId) -> Result<FolderPath, MountFailure> {
        self.runtime.block_on(self.disks.mount(id))
    }

    fn eject(&self, id: &VolumeId) -> Result<(), EjectFailure> {
        self.runtime.block_on(self.disks.eject(id))
    }

    fn force_eject(&self, id: &VolumeId) -> Result<(), EjectFailure> {
        self.runtime.block_on(self.disks.force_eject(id))
    }

    fn watch(&self, changed: Changed) -> Result<WatchGuard, VolumeError> {
        // Subscribed before this returns, so a list made after it misses nothing.
        let mut changes = self
            .runtime
            .block_on(self.disks.changes())
            .map_err(|_| VolumeError::Unavailable)?;
        let task = self.runtime.spawn(async move {
            while changes.next().await.is_some() {
                changed();
            }
        });
        Ok(WatchGuard::new(Stops(task)))
    }
}
