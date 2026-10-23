//! The cloud drives behind a trait: which there are, where each file in them stands, the asks a
//! person makes (Download Now, Remove Download, Keep Downloaded) and a stream of news.
//!
//! The real service is the desktop crate's; [`NoClouds`] is a computer with none, and with the
//! `testing` feature [`FakeClouds`] keeps drives in memory and lets a test move a download along.

use std::any::Any;
use std::fmt;

use files_core::{CloudDrive, CloudItem, CloudNews, ItemRef, Keeping, Refused, SavedCopies};

#[cfg(any(test, feature = "testing"))]
mod fake;

pub use crate::error::CloudError;

#[cfg(any(test, feature = "testing"))]
pub use fake::{CloudCall, FakeClouds, NO_FILE_FORMAT_WORDS, OFFLINE_WORDS};

/// What a [`Clouds`] calls with each piece of news, from any thread.
pub type News = Box<dyn Fn(CloudNews) + Send + Sync>;

/// What keeps a watch going: dropping it stops the watch.
pub struct CloudGuard {
    /// Held only to be dropped with the guard.
    _held: Box<dyn Any + Send>,
}

impl CloudGuard {
    /// A guard that keeps `held` alive until it is dropped.
    pub fn new(held: impl Any + Send) -> CloudGuard {
        CloudGuard {
            _held: Box::new(held),
        }
    }
}

impl fmt::Debug for CloudGuard {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CloudGuard")
    }
}

/// The cloud drives of the computer. Every call may block, so the caller makes them on a thread
/// of its own.
pub trait Clouds: Send + Sync {
    /// The drives now; `Unavailable` while the service is not there.
    fn drives(&self) -> Result<Vec<CloudDrive>, CloudError>;

    /// Where each of `items` stands. An item on no drive, or not there, is left out.
    fn items(&self, items: &[ItemRef]) -> Result<Vec<(ItemRef, CloudItem)>, CloudError>;

    /// "Download Now": the items that were not taken, with the reasons.
    fn download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, CloudError>;

    /// "Remove Download": the items that were not taken, with the reasons.
    fn remove_download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, CloudError>;

    /// "Save a Copy": saves each online document of `items` as a file next to it. Anything else
    /// is refused, with the reason.
    fn save_copy(&self, items: &[ItemRef]) -> Result<SavedCopies, CloudError>;

    /// "Keep Downloaded" on or off: the items that were not taken, with the reasons.
    fn keep(&self, items: &[ItemRef], keeping: Keeping) -> Result<Vec<Refused>, CloudError>;

    /// Calls `news` with every change until the guard drops. The service going away or coming
    /// back is news too (`CloudNews::Drives`): the caller asks for the drives again.
    fn watch(&self, news: News) -> Result<CloudGuard, CloudError>;
}

/// A computer with no cloud drives: the window shows none.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoClouds;

impl Clouds for NoClouds {
    fn drives(&self) -> Result<Vec<CloudDrive>, CloudError> {
        Err(CloudError::Unavailable)
    }

    fn items(&self, _items: &[ItemRef]) -> Result<Vec<(ItemRef, CloudItem)>, CloudError> {
        Err(CloudError::Unavailable)
    }

    fn download(&self, _items: &[ItemRef]) -> Result<Vec<Refused>, CloudError> {
        Err(CloudError::Unavailable)
    }

    fn remove_download(&self, _items: &[ItemRef]) -> Result<Vec<Refused>, CloudError> {
        Err(CloudError::Unavailable)
    }

    fn save_copy(&self, _items: &[ItemRef]) -> Result<SavedCopies, CloudError> {
        Err(CloudError::Unavailable)
    }

    fn keep(&self, _items: &[ItemRef], _keeping: Keeping) -> Result<Vec<Refused>, CloudError> {
        Err(CloudError::Unavailable)
    }

    fn watch(&self, _news: News) -> Result<CloudGuard, CloudError> {
        Err(CloudError::Unavailable)
    }
}
