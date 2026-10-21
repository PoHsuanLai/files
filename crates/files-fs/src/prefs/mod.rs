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

/// Where each folder's view and sort are kept. Real: [`TomlPrefs`]; for tests: `FakePrefs`
/// (feature `testing`).
pub trait PrefsStore: Send + Sync {
    /// How `folder` was last shown; the defaults for a folder never changed (or when the store
    /// can't be read).
    fn load(&self, folder: &FolderPath) -> FolderPrefs;

    /// Remembers that `folder` is shown with `prefs`.
    fn save(&self, folder: &FolderPath, prefs: FolderPrefs) -> Result<(), PrefsError>;
}
