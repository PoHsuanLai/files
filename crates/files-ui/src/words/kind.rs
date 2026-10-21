//! What kind of thing an entry is, in the words of Finder's Kind column.
//!
//! A MIME type's own description ("PDF document") is quire's and the desktop's to give once a
//! lookup exists; until then a file's kind comes from its extension.

use files_core::{Entry, EntryKind, LinkTo, Special};

/// The Kind column's words for `entry`: `Folder`, `Alias`, `PDF file`, or `Document` for a file
/// with no extension.
pub fn kind_words(entry: &Entry) -> String {
    match &entry.kind {
        EntryKind::Folder => "Folder".to_owned(),
        EntryKind::File(_) => file_words(entry),
        EntryKind::Link(LinkTo::Broken) => "Broken alias".to_owned(),
        EntryKind::Link(_) => "Alias".to_owned(),
        EntryKind::Special(Special::Fifo) => "Pipe".to_owned(),
        EntryKind::Special(Special::Socket) => "Socket".to_owned(),
        EntryKind::Special(Special::Device) => "Device".to_owned(),
        EntryKind::Special(_) => "Item".to_owned(),
        // A kind files-core adds later is just an item until this names it.
        _ => "Item".to_owned(),
    }
}

/// `PDF file` for `report.pdf`; `Document` when the name has no extension.
fn file_words(entry: &Entry) -> String {
    match entry.name.extension() {
        Some(extension) => format!("{} file", extension.to_string_lossy().to_uppercase()),
        None => "Document".to_owned(),
    }
}
