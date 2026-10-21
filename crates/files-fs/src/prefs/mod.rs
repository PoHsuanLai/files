//! How each folder was last shown, remembered between runs.
//!
//! [`PrefsStore`] is the seam: [`TomlPrefs`] keeps the book in one file in the app's state folder,
//! `FakePrefs` (feature `testing`) keeps it in memory. The book itself ([`PrefBook`]) is pure data
//! and text, so what it keeps and what it forgets is tested without a disk.

mod book;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod toml_store;

use files_core::{FolderPath, FolderPrefs};

use crate::error::PrefsError;

pub use book::{KEPT, PrefBook};
#[cfg(any(test, feature = "testing"))]
pub use fake::FakePrefs;
pub use toml_store::TomlPrefs;

/// A store that remembers nothing: every folder opens with the defaults. What a window has until
/// it is given a real one.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoPrefs;

impl PrefsStore for NoPrefs {
    fn load(&self, _folder: &FolderPath) -> FolderPrefs {
        FolderPrefs::default()
    }

    fn save(&self, _folder: &FolderPath, _prefs: FolderPrefs) -> Result<(), PrefsError> {
        Ok(())
    }
}

/// Where each folder's view and sort are kept. Real: [`TomlPrefs`]; for tests: `FakePrefs`
/// (feature `testing`).
pub trait PrefsStore: Send + Sync {
    /// How `folder` was last shown; the defaults for a folder never changed (or when the store
    /// can't be read).
    fn load(&self, folder: &FolderPath) -> FolderPrefs;

    /// Remembers that `folder` is shown with `prefs`.
    fn save(&self, folder: &FolderPath, prefs: FolderPrefs) -> Result<(), PrefsError>;
}
