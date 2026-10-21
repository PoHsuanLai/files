//! What opening an entry does.

use files_core::{Entry, EntryKind, LinkTo};

/// What opening an entry does.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Opens {
    /// It is a folder, or a link to one: opening goes into it.
    Folder,
    /// It is a file, or a link to one: opening hands it to an app.
    File,
    /// Nothing: a pipe, a socket, a device or a link that leads nowhere.
    Nothing,
}

/// What opening `entry` does.
pub fn opens(entry: &Entry) -> Opens {
    match &entry.kind {
        EntryKind::Folder | EntryKind::Link(LinkTo::Folder) => Opens::Folder,
        EntryKind::File(_) | EntryKind::Link(LinkTo::File(_)) => Opens::File,
        EntryKind::Link(_) | EntryKind::Special(_) => Opens::Nothing,
        // A kind files-core adds later opens nothing until this names it.
        _ => Opens::Nothing,
    }
}
