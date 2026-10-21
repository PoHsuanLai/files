//! The model's rows as quire's table wants them: columns, the sort, the cursor and the selection,
//! by row index. Pure.

use std::collections::HashMap;

use ds::base::selection::{PickHow, RowSelection};
use ds::components::lists::table::model::SortDirection;
use files_core::{FileName, SortKey, SortOrder};
use files_model::{Browser, Modifier};

/// The table's columns, left to right.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Column {
    /// The name, with its icon. Pinned: it stays while the others scroll sideways.
    Name,
    /// When the entry last changed.
    Modified,
    /// How big a file is.
    Size,
    /// What the entry is.
    Kind,
}

impl Column {
    /// Every column, in order.
    pub const ALL: [Column; 4] = [Column::Name, Column::Modified, Column::Size, Column::Kind];

    /// The header's words.
    pub fn title(self) -> &'static str {
        match self {
            Column::Name => "Name",
            Column::Modified => "Date Modified",
            Column::Size => "Size",
            Column::Kind => "Kind",
        }
    }

    /// The column a pressed sort key is drawn under.
    pub fn of_key(key: SortKey) -> Column {
        match key {
            SortKey::Name => Column::Name,
            SortKey::Modified => Column::Modified,
            SortKey::Size => Column::Size,
            SortKey::Kind => Column::Kind,
            // A key files-core adds later sorts like the name until a column is drawn for it.
            _ => Column::Name,
        }
    }

    /// The key a press on this column's header sorts by.
    pub fn key(self) -> SortKey {
        match self {
            Column::Name => SortKey::Name,
            Column::Modified => SortKey::Modified,
            Column::Size => SortKey::Size,
            Column::Kind => SortKey::Kind,
        }
    }
}

/// The table's direction for a model's order.
pub fn direction_of(order: SortOrder) -> SortDirection {
    match order {
        SortOrder::Ascending => SortDirection::Ascending,
        SortOrder::Descending => SortDirection::Descending,
    }
}

/// The model's order for a table's direction.
pub fn order_of(direction: SortDirection) -> SortOrder {
    match direction {
        SortDirection::Ascending => SortOrder::Ascending,
        SortDirection::Descending => SortOrder::Descending,
    }
}

/// What a click's reach means to the model's selection.
pub fn modifier_of(how: PickHow) -> Modifier {
    match how {
        PickHow::Replace => Modifier::None,
        PickHow::Toggle => Modifier::Toggle,
        PickHow::Extend => Modifier::Extend,
    }
}

/// The index of each row's name, to look many up at once.
fn indexes(browser: &Browser) -> HashMap<&FileName, usize> {
    browser
        .rows()
        .iter()
        .enumerate()
        .map(|(at, row)| (&row.name, at))
        .collect()
}

/// The row the keyboard is on.
pub fn cursor_of(browser: &Browser) -> Option<usize> {
    let name = browser.cursor()?;
    browser.rows().iter().position(|row| row.name == *name)
}

/// The selected rows by index, with the anchor of a Shift-click and the cursor.
pub fn selection_of(browser: &Browser) -> RowSelection {
    let index = indexes(browser);
    let at = |name: Option<&FileName>| name.and_then(|name| index.get(name).copied());
    let selection = browser.selection();
    let rows = selection
        .names()
        .filter_map(|name| index.get(name).copied());
    RowSelection::of_rows(at(selection.anchor()), at(selection.cursor()), rows)
}
