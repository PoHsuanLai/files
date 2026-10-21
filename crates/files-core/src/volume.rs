//! Disks: the volumes the sidebar lists under Locations, and why one would not eject or open.

use std::fmt;

use crate::FolderPath;

/// What the system calls a volume, kept opaque: the same disk has the same id for as long as it
/// is attached.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VolumeId(String);

impl VolumeId {
    /// The volume the system calls `text`.
    pub fn new(text: impl Into<String>) -> VolumeId {
        VolumeId(text.into())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// A volume's name as the person reads it in the sidebar.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct VolumeName(String);

impl VolumeName {
    /// The name `text`.
    pub fn new(text: impl Into<String>) -> VolumeName {
        VolumeName(text.into())
    }

    /// The name as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for VolumeName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// What sort of disk a volume is, for its glyph and its place in the list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum VolumeKind {
    /// A drive or card that can be taken away (a USB stick, an SD card).
    External,
    /// A disk built into the computer that is not the system's own.
    Internal,
    /// An optical disc.
    Optical,
}

/// Whether a volume is available as a folder, and where.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Mount {
    /// Its contents are at this folder.
    At(FolderPath),
    /// It is attached but not opened yet; opening it mounts it.
    Not,
}

/// Whether the sidebar offers to eject a volume.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Eject {
    /// An eject button is shown.
    Allowed,
    /// The disk stays (the computer's own).
    Never,
}

/// One disk the sidebar lists.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Volume {
    /// What the system calls it.
    pub id: VolumeId,
    /// What the person reads.
    pub name: VolumeName,
    /// What sort of disk it is.
    pub kind: VolumeKind,
    /// Whether it is open as a folder.
    pub mount: Mount,
    /// Whether it can be ejected.
    pub eject: Eject,
}

impl Volume {
    /// A volume that is not mounted and cannot be ejected.
    pub fn new(id: VolumeId, name: VolumeName, kind: VolumeKind) -> Volume {
        Volume {
            id,
            name,
            kind,
            mount: Mount::Not,
            eject: Eject::Never,
        }
    }

    /// The same volume mounted at `folder`.
    pub fn mounted_at(self, folder: FolderPath) -> Volume {
        Volume {
            mount: Mount::At(folder),
            ..self
        }
    }

    /// The same volume with an eject button.
    pub fn ejectable(self) -> Volume {
        Volume {
            eject: Eject::Allowed,
            ..self
        }
    }

    /// The folder it is mounted at, when it is.
    pub fn folder(&self) -> Option<&FolderPath> {
        match &self.mount {
            Mount::At(folder) => Some(folder),
            Mount::Not => None,
        }
    }

    /// Whether `folder` is the volume's own folder or inside it.
    pub fn holds(&self, folder: &FolderPath) -> bool {
        self.folder()
            .is_some_and(|mount| folder.as_path().starts_with(mount.as_path()))
    }
}

/// A program that has the disk open, as the person knows it.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct ProgramName(String);

impl ProgramName {
    /// The program called `text`.
    pub fn new(text: impl Into<String>) -> ProgramName {
        ProgramName(text.into())
    }

    /// The name as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Why a volume was not ejected.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EjectFailure {
    /// Something has files on it open; the programs are listed when they could be found.
    #[error("One or more programs may be using the disk.")]
    InUse(Vec<ProgramName>),
    /// The system would not let this person eject it.
    #[error("You don't have permission to eject the disk.")]
    NotAllowed,
    /// The disk is gone already.
    #[error("The disk is not there any more.")]
    Gone,
    /// Anything else the system reported.
    #[error("The disk couldn't be ejected.")]
    Other,
}

/// Why a volume was not opened.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MountFailure {
    /// The system would not let this person open it.
    #[error("You don't have permission to open the disk.")]
    NotAllowed,
    /// The disk is gone.
    #[error("The disk is not there any more.")]
    Gone,
    /// Anything else the system reported.
    #[error("The disk couldn't be opened.")]
    Other,
}

/// What went wrong with a volume, to be told to the person.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VolumeTrouble {
    /// It was not ejected.
    NotEjected(EjectFailure),
    /// It was not opened.
    NotOpened(MountFailure),
}
