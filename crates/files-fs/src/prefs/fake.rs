//! A prefs store in memory.

use std::sync::{Mutex, MutexGuard, PoisonError};

use files_core::{FolderPath, FolderPrefs};

use super::{PrefBook, PrefsStore};
use crate::error::PrefsError;

/// A [`PrefsStore`] that keeps its [`PrefBook`] in memory; share one between two windows to
/// stand for a relaunch.
#[derive(Debug, Default)]
pub struct FakePrefs {
    book: Mutex<PrefBook>,
}

impl FakePrefs {
    /// A store that remembers nothing yet.
    pub fn new() -> FakePrefs {
        FakePrefs::default()
    }

    fn book(&self) -> MutexGuard<'_, PrefBook> {
        self.book.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl PrefsStore for FakePrefs {
    fn load(&self, folder: &FolderPath) -> FolderPrefs {
        self.book().get(folder).unwrap_or_default()
    }

    fn save(&self, folder: &FolderPath, prefs: FolderPrefs) -> Result<(), PrefsError> {
        self.book().set(folder.clone(), prefs);
        Ok(())
    }
}
