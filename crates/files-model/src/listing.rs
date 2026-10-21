//! What the folder holds, as last read.

use files_core::Entry;

use crate::error::ListFailure;
use crate::input::Change;

/// The folder's contents as last loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Listing {
    /// Waiting for the first answer.
    Loading,
    /// A search is under way: the entries found so far, in no particular order.
    Searching(Vec<Entry>),
    /// Every entry, in no particular order.
    Loaded(Vec<Entry>),
    /// It could not be read.
    Failed(ListFailure),
}

impl Listing {
    /// The entries known so far; none while loading or after a failure.
    pub fn entries(&self) -> &[Entry] {
        match self {
            Listing::Loaded(entries) | Listing::Searching(entries) => entries,
            Listing::Loading | Listing::Failed(_) => &[],
        }
    }
}

/// `entries` with `change` applied: new and updated entries replace the one of the same name, or
/// join the end.
pub(crate) fn apply(mut entries: Vec<Entry>, change: Change) -> Vec<Entry> {
    match change {
        Change::Added(entry) | Change::Updated(entry) => {
            match entries.iter_mut().find(|e| e.name == entry.name) {
                Some(slot) => *slot = entry,
                None => entries.push(entry),
            }
        }
        Change::Removed(name) => entries.retain(|e| e.name != name),
        Change::Rescan => {}
    }
    entries
}
