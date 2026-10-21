//! What the context menu offers: pure, so a table can say it.

use files_core::Location;
use files_model::{Availability, Browser, Input};

/// One command of the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Open the selection.
    Open,
    /// Move the selection to the Trash.
    MoveToTrash,
    /// Rename the selected item.
    Rename,
    /// Copy the selection.
    Copy,
    /// Paste what was copied into this folder.
    Paste,
    /// Make a new folder.
    NewFolder,
    /// Put the selection back where it came from (in the Trash).
    PutBack,
    /// Erase the selection for good (in the Trash).
    DeleteImmediately,
    /// Erase everything in the Trash for good.
    EmptyTrash,
}

impl Command {
    /// The menu item's words.
    pub fn title(self) -> &'static str {
        match self {
            Command::Open => "Open",
            Command::MoveToTrash => "Move to Trash",
            Command::Rename => "Rename",
            Command::Copy => "Copy",
            Command::Paste => "Paste",
            Command::NewFolder => "New Folder",
            Command::PutBack => "Put Back",
            Command::DeleteImmediately => "Delete Immediately\u{2026}",
            Command::EmptyTrash => "Empty Trash\u{2026}",
        }
    }

    /// What picking it asks of the browser.
    pub fn input(self) -> Input {
        match self {
            Command::Open => Input::Open,
            Command::MoveToTrash => Input::MoveToTrash,
            Command::Rename => Input::BeginRename,
            Command::Copy => Input::Copy,
            Command::Paste => Input::Paste,
            Command::NewFolder => Input::NewFolder,
            Command::PutBack => Input::PutBack,
            Command::DeleteImmediately => Input::DeleteImmediately,
            Command::EmptyTrash => Input::EmptyTrash,
        }
    }
}

/// Where the press was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// On a row (which the press has selected).
    Row,
    /// On the folder's background.
    Background,
}

/// The menu for a press `over` something in `browser`: groups of commands, each with whether it
/// can be picked now.
pub fn groups_for(browser: &Browser, over: Over) -> Vec<Vec<(Command, Availability)>> {
    let selected = browser.selection().len();
    let some = if selected > 0 {
        Availability::Available
    } else {
        Availability::Unavailable
    };
    let one = if selected == 1 {
        Availability::Available
    } else {
        Availability::Unavailable
    };
    let rows = if browser.rows().is_empty() {
        Availability::Unavailable
    } else {
        Availability::Available
    };
    match (browser.location(), over) {
        (Location::Trash, Over::Row) => vec![vec![
            (Command::PutBack, some),
            (Command::DeleteImmediately, some),
        ]],
        (Location::Trash, Over::Background) => vec![vec![(Command::EmptyTrash, rows)]],
        (_, Over::Row) => vec![
            vec![(Command::Open, some)],
            vec![(Command::MoveToTrash, some), (Command::Rename, one)],
            vec![(Command::Copy, some), (Command::Paste, browser.can_paste())],
            vec![(Command::NewFolder, Availability::Available)],
        ],
        (_, Over::Background) => vec![
            vec![(Command::NewFolder, Availability::Available)],
            vec![(Command::Paste, browser.can_paste())],
        ],
    }
}
