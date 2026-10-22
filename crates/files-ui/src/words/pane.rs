//! What the preview pane says of the selection, as rows of label and words.

use std::time::SystemTime;

use files_core::{Entry, EntryKind, Info, LinkTo};

use super::{InfoRow, items_words, kind_words, size_words};

/// The title of the pane when several items are selected: `3 items`.
pub fn many_words(count: usize) -> String {
    format!("{count} items")
}

/// The rows under a lone item's preview, in order: Kind, Size (a file) or Contents (a folder),
/// Created (when the filesystem says), Modified, and Dimensions (what anyview reports of a
/// picture). `entry` is the row, `info` the facts read for it when they have arrived, and `when`
/// writes a date.
pub fn pane_rows(
    entry: &Entry,
    info: Option<&Info>,
    dimensions: Option<String>,
    when: impl Fn(Option<SystemTime>) -> String,
) -> Vec<InfoRow> {
    let mut rows = vec![InfoRow::new("Kind", kind_words(entry))];
    if let EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) = &entry.kind {
        rows.push(InfoRow::new("Size", size_words(*size)));
    }
    if let Some(count) = info.and_then(|info| info.items) {
        rows.push(InfoRow::new("Contents", items_words(count)));
    }
    if let Some(created) = info.and_then(|info| info.created) {
        rows.push(InfoRow::new("Created", when(Some(created))));
    }
    let modified = entry
        .modified
        .or_else(|| info.and_then(|info| info.modified));
    rows.push(InfoRow::new("Modified", when(modified)));
    if let Some(dimensions) = dimensions {
        rows.push(InfoRow::new("Dimensions", dimensions));
    }
    rows
}
