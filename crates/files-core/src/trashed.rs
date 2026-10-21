//! An item that is in the Trash.

use std::ffi::{OsStr, OsString};
use std::path::Path;
use std::time::SystemTime;

use crate::{EntryKind, FileName, ItemRef};

/// The Trash's own handle on an item: opaque to everyone but the crate that made it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TrashId(OsString);

impl TrashId {
    /// A handle from the Trash's own text.
    pub fn new(id: impl Into<OsString>) -> TrashId {
        TrashId(id.into())
    }

    /// The handle as the Trash wrote it.
    pub fn as_os_str(&self) -> &OsStr {
        &self.0
    }
}

/// One item in the Trash: where it came from, when it was thrown away and what it is.
///
/// This is also what a move to the Trash gives back, so the move can be undone by putting this
/// item back.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct TrashedItem {
    /// The Trash's handle on it.
    pub id: TrashId,
    /// Where it was before it was thrown away.
    pub original: ItemRef,
    /// When it was thrown away.
    pub deleted: SystemTime,
    /// What it is.
    pub kind: EntryKind,
}

impl TrashedItem {
    /// An item the Trash holds under `id`.
    pub fn new(
        id: TrashId,
        original: ItemRef,
        deleted: SystemTime,
        kind: EntryKind,
    ) -> TrashedItem {
        TrashedItem {
            id,
            original,
            deleted,
            kind,
        }
    }

    /// The name it has inside the Trash, which is unique there: its original name, with a number
    /// added when two items of the same name were thrown away. It is the handle's file name
    /// without the `.trashinfo` ending; `None` when the handle is not shaped like that.
    pub fn stored_name(&self) -> Option<FileName> {
        let stem = Path::new(self.id.as_os_str()).file_stem()?;
        FileName::new(stem).ok()
    }
}
