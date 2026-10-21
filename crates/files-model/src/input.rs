//! What can happen to a browser.

use std::time::Duration;

use files_core::{Entry, FileName, FolderPrefs, Location, Sort, ViewMode};

use crate::listing::ListFailure;

/// The time, as the caller reads a monotonic clock: since an arbitrary origin, never going back.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub struct Now(pub Duration);

/// Which other rows a click or a key press keeps.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Modifier {
    /// Plain: only this row.
    None,
    /// Shift: everything from the anchor to this row.
    Extend,
    /// The primary key (⌘): this row is added or removed, the rest stays.
    Toggle,
}

/// An arrow or jump key in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Direction {
    /// One row up.
    Up,
    /// One row down.
    Down,
    /// The first row.
    Home,
    /// The last row.
    End,
}

/// One live change to the folder being watched.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Change {
    /// A new entry appeared.
    Added(Entry),
    /// An entry is gone.
    Removed(FileName),
    /// An entry changed (its size, its date).
    Updated(Entry),
    /// Changes were missed; read the folder again.
    Rescan,
}

/// Everything that can happen to a browser: what the person did and what the caller found out.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Input {
    /// The answer to `Effect::List`.
    Listed(Location, Result<Vec<Entry>, ListFailure>),
    /// A live change from `Effect::Watch`.
    Changed(Location, Change),
    /// The answer to `Effect::LoadPrefs`.
    PrefsLoaded(Location, FolderPrefs),
    /// Go to a place.
    Go(Location),
    /// Go back (⌘[).
    Back,
    /// Go forward (⌘]).
    Forward,
    /// Go to the folder that holds this one (⌘↑).
    Enclosing,
    /// Read the folder again.
    Reload,
    /// Open the selection (Return, double-click, ⌘↓).
    Open,
    /// A click on a row.
    Click(FileName, Modifier),
    /// Select every row (⌘A).
    SelectAll,
    /// Select nothing.
    SelectNone,
    /// An arrow or jump key.
    MoveCursor(Direction, Modifier),
    /// A typed character, to jump to a name.
    Type(char),
    /// Start renaming the selected entry.
    BeginRename,
    /// The rename field's text changed.
    EditDraft(String),
    /// Finish renaming (Return).
    CommitRename,
    /// Stop renaming (Escape).
    CancelRename,
    /// Show the folder in this view.
    SetView(ViewMode),
    /// Sort the folder this way.
    SetSort(Sort),
    /// Show or leave out hidden files.
    ToggleHidden,
}
