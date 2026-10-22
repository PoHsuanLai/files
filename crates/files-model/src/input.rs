//! What can happen to a browser.

use std::time::Duration;

use files_core::{
    EjectFailure, Entry, FileName, FolderPath, FolderPrefs, Info, ItemRef, Location, MountFailure,
    Resolution, Sort, TagColour, ViewMode, Volume, VolumeId,
};

use crate::app::AppPick;
use crate::asked::Bring;
use crate::error::ListFailure;
use crate::ops::{DropHow, OpEvent};
use crate::tabs::TabId;

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
    /// A batch of the files a search found, from `Effect::Find`.
    Found(Location, Vec<Entry>),
    /// The search of a place has looked everywhere it will.
    SearchDone(Location),
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
    /// Put up the Go to Folder sheet (⇧⌘G), with the folder shown as its text.
    GoToFolder,
    /// The Go to Folder field's text changed.
    GoToTyped(String),
    /// Return in the Go to Folder field: go to the folder typed, if it is there.
    GoToSubmit,
    /// Put the Go to Folder sheet away (Escape, Cancel).
    GoToCancel,
    /// The answer to `Effect::CheckFolder`.
    FolderChecked(FolderPath, Result<(), ListFailure>),
    /// Read the folder again.
    Reload,
    /// Open the selection (Return, double-click, ⌘↓).
    Open,
    /// Open the selection with this app (Open With, in the context menu).
    OpenWith(AppPick),
    /// Put up the sheet that lists every installed app, to pick one to open the selection with
    /// (Open With, Other).
    ChooseApp,
    /// Put the sheet of apps away.
    CloseChooseApp,
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
    /// Open Get Info on the folder shown, whatever is selected (the background's menu).
    GetInfoFolder,
    /// The answer to `Effect::ReadInfo`.
    InfoRead(ItemRef, Result<Info, ListFailure>),
    /// Put Get Info away (Escape, Done).
    CloseInfo,
    /// Show the preview pane, or put it away (Shift+Command+P).
    TogglePreview,
    /// The answer to `Effect::ReadDetails`.
    DetailsRead(ItemRef, Result<Info, ListFailure>),
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
    /// Put the full paths of the selection on the clipboard as text, one per line
    /// (Option+Command+C).
    CopyAsPathname,
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
    /// The disks the sidebar lists changed (one came or went, one was opened or closed).
    Volumes(Vec<Volume>),
    /// Open a disk from the sidebar: go to it, mounting it first when it is not.
    OpenVolume(VolumeId),
    /// The answer to `Effect::MountVolume`.
    Mounted(VolumeId, Result<FolderPath, MountFailure>),
    /// Eject a disk (its button in the sidebar).
    Eject(VolumeId),
    /// The answer to `Effect::EjectVolume`.
    Ejected(VolumeId, Result<(), EjectFailure>),
    /// The answer to `Effect::ForceEjectVolume`. A failure is only told, never asked again.
    ForceEjected(VolumeId, Result<(), EjectFailure>),
    /// Move these items to the Trash, wherever they are (another program asked).
    TrashItems(Vec<ItemRef>),
    /// Make a folder in `parent`, called `name` or, when none is given, `untitled folder` (another
    /// program asked). The window does not start renaming it.
    MakeFolder {
        /// The folder to make it in.
        parent: FolderPath,
        /// Its name, if the asker gave one.
        name: Option<FileName>,
    },
    /// Copy or move these items to a folder, wherever they are (another program asked).
    Bring {
        /// What to bring.
        sources: Vec<ItemRef>,
        /// The folder to bring it to.
        destination: FolderPath,
        /// Copy or move.
        how: Bring,
    },
    /// Ask whether to empty the Trash, from any place (another program asked; the person
    /// answers).
    AskEmptyTrash,
    /// Put the tag of this colour on the selection, or take it off when every selected item has
    /// it already.
    Tag(TagColour),
    /// Open a new tab on the place the front tab shows (⌘T). Handled by `Tabs`.
    NewTab,
    /// Open the selected folder in a new tab (the context menu). Handled by `Tabs`.
    OpenInNewTab,
    /// Close the front tab, or the window when it is the last (⌘W). Handled by `Tabs`.
    CloseTab,
    /// Show the tab to the right of the front one, the first after the last (⇧⌘]). Handled by
    /// `Tabs`.
    NextTab,
    /// Show the tab to the left of the front one, the last before the first (⇧⌘[). Handled by
    /// `Tabs`.
    PreviousTab,
    /// Show this tab. Handled by `Tabs`.
    ShowTab(TabId),
}
