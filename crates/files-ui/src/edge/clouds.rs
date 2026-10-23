//! The cloud drives: the thread that serves the window's asks of the `Clouds` seam.
//!
//! One thread joins the seam's news, lists the drives at the start and again whenever the seam
//! says they may have changed, reads where items stand, and carries out Download Now, Remove
//! Download and Keep Downloaded in order, so an ask and the reads after it never cross. The
//! drives are told to every tab only when they differ from what was last told: a service that
//! is not there tells nothing, one that goes away tells an empty list once (no drives, no
//! errors), and one that comes back tells its drives.

use std::sync::Arc;

use files_core::{CloudDrive, CloudNews, CloudWords, ItemRef, Keeping, Refused, SavedCopies};
use files_fs::Clouds;
use files_model::{Input, TabId};
use tokio::sync::mpsc::{UnboundedSender, unbounded_channel};

use super::Heard;

/// What the cloud thread is asked.
enum Ask {
    News(CloudNews),
    Read(TabId, Vec<ItemRef>),
    Download(TabId, Vec<ItemRef>),
    Remove(TabId, Vec<ItemRef>),
    Keep(TabId, Vec<ItemRef>, Keeping),
    Copy(TabId, Vec<ItemRef>),
    Stop,
}

/// The cloud thread's door. Dropping it stops the thread and the watch.
pub(super) struct CloudDrives {
    asks: UnboundedSender<Ask>,
}

/// An ask that could not be made is refused for every item, in the seam's own words.
fn all_refused(items: Vec<ItemRef>, why: &impl std::fmt::Display) -> Vec<Refused> {
    items
        .into_iter()
        .map(|item| Refused {
            item,
            words: CloudWords::new(why.to_string()),
        })
        .collect()
}

impl CloudDrives {
    /// Starts the thread over `clouds`. Without a thread (the system would not start one) the
    /// window simply shows no cloud drives.
    pub(super) fn start(clouds: Arc<dyn Clouds>, heard: UnboundedSender<Heard>) -> CloudDrives {
        let (asks, mut inbox) = unbounded_channel::<Ask>();
        let news = asks.clone();
        let _ = std::thread::Builder::new()
            .name("files-clouds".to_owned())
            .spawn(move || {
                let tell = |what: Heard| {
                    // A closed channel means the window is gone.
                    let _ = heard.send(what);
                };
                let mut told: Vec<CloudDrive> = Vec::new();
                let mut relist = || {
                    let now = clouds.drives().unwrap_or_default();
                    if now != told {
                        told = now.clone();
                        tell(Heard::All(Input::Clouds(now)));
                    }
                };
                let guard = clouds.watch(Box::new(move |heard_of| {
                    let _ = news.send(Ask::News(heard_of));
                }));
                relist();
                while let Some(ask) = inbox.blocking_recv() {
                    match ask {
                        Ask::News(CloudNews::Drives) => relist(),
                        Ask::News(CloudNews::Items(_, rows)) => {
                            tell(Heard::All(Input::CloudItems(rows)));
                        }
                        // News a later seam sends that this build does not read.
                        Ask::News(_) => {}
                        Ask::Read(tab, items) => {
                            if let Ok(rows) = clouds.items(&items) {
                                tell(Heard::Tab(tab, Input::CloudItems(rows)));
                            }
                        }
                        Ask::Download(tab, items) => {
                            let result = clouds.download(&items);
                            answer(&tell, tab, items, result);
                        }
                        Ask::Remove(tab, items) => {
                            let result = clouds.remove_download(&items);
                            answer(&tell, tab, items, result);
                        }
                        Ask::Keep(tab, items, keeping) => {
                            let result = clouds.keep(&items, keeping);
                            answer(&tell, tab, items, result);
                        }
                        Ask::Copy(tab, items) => {
                            let copies =
                                clouds
                                    .save_copy(&items)
                                    .unwrap_or_else(|error| SavedCopies {
                                        saved: Vec::new(),
                                        refused: all_refused(items, &error),
                                    });
                            tell(Heard::Tab(tab, Input::CopiesSaved(copies)));
                        }
                        Ask::Stop => break,
                    }
                }
                drop(guard);
            });
        CloudDrives { asks }
    }

    /// Reads where `items` stand and tells the tab.
    pub(super) fn read(&self, tab: TabId, items: Vec<ItemRef>) {
        let _ = self.asks.send(Ask::Read(tab, items));
    }

    /// Download Now.
    pub(super) fn download(&self, tab: TabId, items: Vec<ItemRef>) {
        let _ = self.asks.send(Ask::Download(tab, items));
    }

    /// Remove Download.
    pub(super) fn remove(&self, tab: TabId, items: Vec<ItemRef>) {
        let _ = self.asks.send(Ask::Remove(tab, items));
    }

    /// Save a Copy of online documents as files.
    pub(super) fn copy(&self, tab: TabId, items: Vec<ItemRef>) {
        let _ = self.asks.send(Ask::Copy(tab, items));
    }

    /// Keep Downloaded, on or off.
    pub(super) fn keep(&self, tab: TabId, items: Vec<ItemRef>, keeping: Keeping) {
        let _ = self.asks.send(Ask::Keep(tab, items, keeping));
    }
}

/// Tells the tab how an ask went: the refusals, or every item refused when it could not be made.
fn answer(
    tell: &impl Fn(Heard),
    tab: TabId,
    items: Vec<ItemRef>,
    result: Result<Vec<Refused>, files_fs::CloudError>,
) {
    let refused = match result {
        Ok(refused) => refused,
        Err(error) => all_refused(items, &error),
    };
    tell(Heard::Tab(tab, Input::CloudAnswered(refused)));
}

impl Drop for CloudDrives {
    fn drop(&mut self) {
        let _ = self.asks.send(Ask::Stop);
    }
}
