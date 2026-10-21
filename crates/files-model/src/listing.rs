//! What the folder holds, as last read.

use files_core::Entry;

use crate::input::Change;

/// Why a folder could not be read, in terms a person understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ListFailure {
    /// It isn't there any more.
    #[error("That folder can't be found.")]
    NotFound,
    /// You're not allowed to look inside.
    #[error("You don't have permission to open that folder.")]
    NoPermission,
    /// It's a file, not a folder.
    #[error("That isn't a folder.")]
    NotAFolder,
    /// Anything else.
    #[error("That folder can't be opened.")]
    Other,
}

/// The folder's contents as last loaded.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Listing {
    /// Waiting for the first answer.
    Loading,
    /// Every entry, in no particular order.
    Loaded(Vec<Entry>),
    /// It could not be read.
    Failed(ListFailure),
}

impl Listing {
    /// The entries known so far; none while loading or after a failure.
    pub fn entries(&self) -> &[Entry] {
        match self {
            Listing::Loaded(entries) => entries,
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
