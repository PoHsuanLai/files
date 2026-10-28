//! The table's header words, in the words of Finder's list view.

use files_core::{Location, SortKey};

use crate::rows::Column;

/// The header's words for `column`.
pub fn column_title(column: Column) -> &'static str {
    match column {
        Column::Sort(SortKey::Modified) => "Date Modified",
        Column::Sort(SortKey::Size) => "Size",
        Column::Sort(SortKey::Kind) => "Kind",
        // The name, and a key files-core adds later until a header is written for it.
        Column::Sort(_) => "Name",
        Column::WhereFrom => "Where From",
    }
}

/// The header's words for `column` in `location`: the date in the Trash is when the item was
/// thrown away.
pub fn column_title_in(column: Column, location: &Location) -> &'static str {
    match (column, location) {
        (Column::Sort(SortKey::Modified), Location::Trash) => "Date Deleted",
        _ => column_title(column),
    }
}
