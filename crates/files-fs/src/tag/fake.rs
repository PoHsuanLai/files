//! An in-memory tagger, for tests of the crates above.

use std::collections::HashMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use files_core::{ItemRef, Tags};

use super::Tagger;
use crate::error::TagWriteError;

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

#[derive(Debug, Default)]
struct Book {
    tags: HashMap<ItemRef, Tags>,
    refusing: Option<TagWriteError>,
}

/// A [`Tagger`] that remembers tags in memory. It can be told to refuse writes, as a disk with no
/// extended attributes does.
#[derive(Debug, Default)]
pub struct FakeTagger {
    book: Mutex<Book>,
}

impl FakeTagger {
    /// A tagger where nothing is tagged.
    pub fn new() -> FakeTagger {
        FakeTagger::default()
    }

    /// A tagger that refuses every write with `error`.
    pub fn refusing(error: TagWriteError) -> FakeTagger {
        FakeTagger {
            book: Mutex::new(Book {
                tags: HashMap::new(),
                refusing: Some(error),
            }),
        }
    }

    /// Gives `item` `tags`, whatever the tagger refuses.
    pub fn put(&self, item: ItemRef, tags: Tags) {
        locked(&self.book).tags.insert(item, tags);
    }
}

impl Tagger for FakeTagger {
    fn tags(&self, item: &ItemRef) -> Tags {
        locked(&self.book)
            .tags
            .get(item)
            .cloned()
            .unwrap_or_default()
    }

    fn set(&self, item: &ItemRef, tags: &Tags) -> Result<(), TagWriteError> {
        let mut book = locked(&self.book);
        if let Some(error) = book.refusing {
            return Err(error);
        }
        book.tags.insert(item.clone(), tags.clone());
        Ok(())
    }
}
