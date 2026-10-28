//! Work in progress and questions waiting for an answer.

use files_core::{
    Conflict, FileName, FolderPath, ProgramName, Progress, Summary, TrashedItem, VolumeId,
    VolumeName,
};

/// How a drop was meant, as the window says it (the model never reads modifier keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropHow {
    /// The usual: a move within one disk, a copy across disks.
    Natural,
    /// A copy, whatever the disks (the person held the copy key).
    Copy,
}

/// The change being made now, if any.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Work {
    /// Nothing is being changed.
    #[default]
    Idle,
    /// A change is under way.
    Running {
        /// What it is about.
        summary: Summary,
        /// How far it has come.
        progress: Progress,
    },
}

/// Something the window must ask before it goes on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Question {
    /// Nothing is being asked.
    #[default]
    None,
    /// Whether to erase everything in the Trash for good.
    EmptyTrash,
    /// Whether to erase these items of the Trash for good.
    DeleteForGood(Vec<TrashedItem>),
    /// A name is taken: keep both, replace, skip or stop.
    Conflict(Conflict),
    /// A disk could not be ejected because programs may be using it: whether to eject it anyway.
    ForceEject(BusyDisk),
    /// A file's new name has another extension: whether to use it or keep the old one.
    ChangeExtension(ExtensionChange),
}

/// A rename that changes a file's extension, waiting to be confirmed.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct ExtensionChange {
    /// The folder the file is in.
    pub folder: FolderPath,
    /// Its name now.
    pub from: FileName,
    /// The name the person typed.
    pub to: FileName,
    /// The typed name with the old extension put back; the file's own name when that is taken.
    pub kept: FileName,
}

impl ExtensionChange {
    /// A rename of `from` to `to` in `folder`; `kept` is the name that keeps the old extension.
    pub fn new(
        folder: FolderPath,
        from: FileName,
        to: FileName,
        kept: FileName,
    ) -> ExtensionChange {
        ExtensionChange {
            folder,
            from,
            to,
            kept,
        }
    }

    /// The extension the file has now, without its dot; `None` when it has none.
    pub fn old_extension(&self) -> Option<String> {
        self.from
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned())
    }

    /// The extension the typed name has, without its dot; `None` when it has none.
    pub fn new_extension(&self) -> Option<String> {
        self.to
            .extension()
            .map(|extension| extension.to_string_lossy().into_owned())
    }
}

/// A disk that would not eject because it is in use.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct BusyDisk {
    /// Which disk.
    pub id: VolumeId,
    /// Its name, as the sidebar shows it.
    pub name: VolumeName,
    /// The programs found using it; none when they could not be found.
    pub programs: Vec<ProgramName>,
}

impl BusyDisk {
    /// The disk `id`, shown as `name`, with the `programs` found using it.
    pub fn new(id: VolumeId, name: VolumeName, programs: Vec<ProgramName>) -> BusyDisk {
        BusyDisk { id, name, programs }
    }
}
