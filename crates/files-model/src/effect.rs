//! What the model asks its caller to do.

use std::path::PathBuf;

use crate::app::AppPick;
use files_core::{
    FileName, FolderPath, FolderPrefs, ItemRef, Location, OpFailure, OpPlan, Resolution, Summary,
    TagColour, VolumeId, VolumeName, VolumeTrouble,
};

/// One file to hand to an app: where it is, and what it is called.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpenTarget {
    /// The folder it is in.
    pub folder: FolderPath,
    /// Its name there.
    pub name: FileName,
}

impl OpenTarget {
    /// The file called `name` in `folder`.
    pub fn new(folder: FolderPath, name: FileName) -> OpenTarget {
        OpenTarget { folder, name }
    }

    /// The full path of the file.
    pub fn path(&self) -> PathBuf {
        self.folder.as_path().join(self.name.as_os_str())
    }
}

/// Whether a tag is put on files or taken off them.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Mark {
    /// Put the tag on.
    Add,
    /// Take the tag off.
    Remove,
}

/// Something for the caller to carry out; the answer, if any, comes back as an
/// [`Input`](crate::Input).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Effect {
    /// Read the folder and answer with `Input::Listed`.
    List(Location),
    /// Look through `within` and the folders under it for files tagged `colour`, and answer with
    /// `Input::Found` for each batch and `Input::SearchDone` at the end. Asking again, or
    /// `Effect::Unwatch` of the same place, stops the walk before.
    Find {
        /// The colour looked for.
        colour: TagColour,
        /// The folder the walk starts in.
        within: FolderPath,
    },
    /// Start telling us about changes there, as `Input::Changed`.
    Watch(Location),
    /// Stop telling us about changes there.
    Unwatch(Location),
    /// Hand these files to the apps that open them.
    OpenFiles(Vec<OpenTarget>),
    /// Hand these files to this app, whatever their own default is.
    OpenFilesWith {
        /// What to open.
        targets: Vec<OpenTarget>,
        /// The app to open it with.
        app: AppPick,
    },
    /// Find out whether this is a folder the person may open, and answer with
    /// `Input::FolderChecked`.
    CheckFolder(FolderPath),
    /// Put this text on the clipboard.
    CopyText(String),
    /// Rename a file in a folder; the listing changes when the disk does.
    Rename {
        /// The folder it is in.
        folder: FolderPath,
        /// Its name now.
        from: FileName,
        /// The name it should have.
        to: FileName,
    },
    /// Look up how this folder was last shown and answer with `Input::PrefsLoaded`.
    LoadPrefs(Location),
    /// Remember how this folder is shown.
    SavePrefs(Location, FolderPrefs),
    /// Make a preview of this item for Quick Look; the answer is not an input (a picture is not
    /// something the model holds), the window shows it when it arrives.
    Look(ItemRef),
    /// Read the facts of this item for Get Info and answer with `Input::InfoRead`.
    ReadInfo(ItemRef),
    /// Do this change to the files, after the ones already asked for; the browser hears how it
    /// goes as `Input::Op`.
    Run(OpPlan),
    /// Take back the newest change that can be taken back.
    Undo,
    /// The person's answer to a taken name the work is waiting on.
    Answer(Resolution),
    /// Stop the change under way.
    Cancel,
    /// Tell the person a change failed.
    Tell(OpFailure),
    /// Tell the person a change was made that they can undo (a move to the Trash).
    Announce(Summary),
    /// Open (mount) this disk and answer with `Input::Mounted`.
    MountVolume(VolumeId),
    /// Eject this disk and answer with `Input::Ejected`.
    EjectVolume(VolumeId),
    /// Eject this disk even though programs may be using it (the person said so), and answer with
    /// `Input::ForceEjected`.
    ForceEjectVolume(VolumeId),
    /// Put the tag of `colour` on these items, or take it off them. The listings follow when the
    /// disk does.
    Tag {
        /// What to change.
        items: Vec<ItemRef>,
        /// The tag's colour.
        colour: TagColour,
        /// Put on or take off.
        how: Mark,
    },
    /// Close the window: its last tab was closed.
    CloseWindow,
    /// Tell the person a disk could not be opened or ejected.
    TellVolume {
        /// The disk's name.
        name: VolumeName,
        /// What went wrong.
        trouble: VolumeTrouble,
    },
}
