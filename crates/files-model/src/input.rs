//! What can happen to a browser.

use std::time::Duration;

use files_core::{
    Entry, FileName, FolderPath, FolderPrefs, Info, ItemRef, Location, Resolution, Sort, ViewMode,
};

use crate::listing::ListFailure;
use crate::ops::{DropHow, OpEvent};

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
    /// Go to a folder and select these entries in it (another program asked to show them, as
    /// "Show in Folder" does). Going there is a normal visit, so Back returns; names the listing
    /// does not hold yet are selected when they arrive, until the person selects something.
    Reveal {
        /// The folder to show.
        folder: FolderPath,
        /// The entries to select; the first is the cursor, and so the one scrolled into view.
        names: Vec<FileName>,
    },
    /// Go to a folder, select these entries in it and open Get Info on the first of them (another
    /// program asked for an item's properties). With no names, Get Info is on the folder itself.
    Properties {
        /// The folder to show.
        folder: FolderPath,
        /// The entries to select; the first is the one Get Info is about.
        names: Vec<FileName>,
    },
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
    /// Open Quick Look on the row the keyboard is on, or put it away if it is up (Space, ⌘Y).
    QuickLook,
    /// Put Quick Look away (Escape).
    CloseQuickLook,
    /// Open Get Info on the row the keyboard is on, or on the folder when nothing is (⌘I).
    GetInfo,
    /// The answer to `Effect::ReadInfo`.
    InfoRead(ItemRef, Result<Info, ListFailure>),
    /// Put Get Info away (Escape, Done).
    CloseInfo,
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
    /// Make a new folder here and start renaming it (Shift+Command+N).
    NewFolder,
    /// Move the selection to the Trash (Command+Backspace).
    MoveToTrash,
    /// Put the selected items of the Trash back where they came from.
    PutBack,
    /// Ask whether to erase the selected items of the Trash for good.
    DeleteImmediately,
    /// Ask whether to empty the Trash.
    EmptyTrash,
    /// Yes, to the question that is up (empty the Trash, erase for good).
    Confirm,
    /// Put the question that is up away; a taken name is answered with Stop.
    Dismiss,
    /// The answer to a taken name the work is waiting on.
    Answer(Resolution),
    /// Remember the selection for a paste (Command+C).
    Copy,
    /// Copy what was copied into this folder (Command+V).
    Paste,
    /// Move what was copied into this folder (Option+Command+V).
    PasteMove,
    /// Items were dropped on a folder row of this folder, or on this folder when `onto` is `None`.
    /// The window says whether the copy key was held; the model never reads keys.
    Drop {
        /// What was dropped.
        sources: Vec<ItemRef>,
        /// The folder row it was dropped on.
        onto: Option<FileName>,
        /// How it was meant.
        how: DropHow,
    },
    /// Take back the newest change that can be taken back (Command+Z).
    Undo,
    /// Stop the change under way.
    CancelWork,
    /// A worker's report on a change.
    Op(OpEvent),
}
