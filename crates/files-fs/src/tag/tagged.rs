//! A lister that fills in each entry's tags.

use std::sync::Arc;

use files_core::{Entry, FileName, FolderPath, Info, ItemRef};

use super::Tagger;
use crate::error::ListError;
use crate::list::Lister;

/// A [`Lister`] over another one, with every entry it gives carrying the tags a [`Tagger`] reads.
pub struct Tagged {
    lister: Arc<dyn Lister>,
    tagger: Arc<dyn Tagger>,
}

impl std::fmt::Debug for Tagged {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Tagged")
    }
}

impl Tagged {
    /// `lister`'s entries, tagged by `tagger`.
    pub fn new(lister: Arc<dyn Lister>, tagger: Arc<dyn Tagger>) -> Tagged {
        Tagged { lister, tagger }
    }

    fn mark(&self, folder: &FolderPath, entry: Entry) -> Entry {
        let tags = self
            .tagger
            .tags(&ItemRef::new(folder.clone(), entry.name.clone()));
        entry.with_tags(tags)
    }
}

impl Lister for Tagged {
    fn list(&self, folder: &FolderPath) -> Result<Vec<Entry>, ListError> {
        let entries = self.lister.list(folder)?;
        Ok(entries
            .into_iter()
            .map(|entry| self.mark(folder, entry))
            .collect())
    }

    fn entry(&self, folder: &FolderPath, name: &FileName) -> Result<Entry, ListError> {
        self.lister
            .entry(folder, name)
            .map(|entry| self.mark(folder, entry))
    }

    fn info(&self, item: &ItemRef) -> Result<Info, ListError> {
        self.lister.info(item)
    }
}
