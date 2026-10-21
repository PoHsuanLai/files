//! What can go wrong, in the words a person reads.

use std::io;

/// Why a folder or an entry could not be read.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ListError {
    /// There is nothing at that path.
    #[error("The folder can't be found.")]
    NotFound,
    /// The system won't let this person look.
    #[error("You don't have permission to see this folder.")]
    NoPermission,
    /// The path leads to something that isn't a folder.
    #[error("That isn't a folder.")]
    NotAFolder,
    /// Anything else the system reported.
    #[error("The folder can't be read.")]
    Other(io::ErrorKind),
}

impl ListError {
    /// The error `err` stands for.
    pub fn from_io(err: &io::Error) -> ListError {
        let kind = err.kind();
        if kind == io::ErrorKind::NotFound {
            ListError::NotFound
        } else if kind == io::ErrorKind::PermissionDenied {
            ListError::NoPermission
        } else if kind == io::ErrorKind::NotADirectory {
            ListError::NotAFolder
        } else {
            ListError::Other(kind)
        }
    }
}

/// Why a folder could not be followed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum WatchError {
    /// There is nothing at that path.
    #[error("The folder can't be found.")]
    NotFound,
    /// The system won't let this person look.
    #[error("You don't have permission to see this folder.")]
    NoPermission,
    /// The system can't follow any more folders right now.
    #[error("Too many folders are being followed.")]
    TooMany,
    /// Anything else the system reported.
    #[error("Changes to this folder can't be followed.")]
    Other,
}

/// Why how a folder is shown could not be remembered.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum PrefsError {
    /// The saved file is there but can't be read or understood, so nothing is written over it.
    #[error("How folders are shown can't be remembered right now.")]
    Unusable,
    /// The folder the file lives in can't be made.
    #[error("How folders are shown can't be saved.")]
    Folder(#[source] io::Error),
    /// The file can't be written.
    #[error("How folders are shown can't be saved.")]
    Write(#[source] ds_helpers::WriteError),
}

/// Why a file could not be tagged, in the words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TagWriteError {
    /// The disk keeps no extended attributes, so it keeps no tags.
    #[error("Tags can\u{2019}t be added on this disk.")]
    Unsupported,
    /// The file is gone.
    #[error("That file can\u{2019}t be found.")]
    NotFound,
    /// The system won't let this person change the file.
    #[error("You don\u{2019}t have permission to tag that file.")]
    NoPermission,
    /// Anything else the system reported.
    #[error("That file can\u{2019}t be tagged.")]
    Other(io::ErrorKind),
}

impl TagWriteError {
    /// The error `err` stands for.
    pub fn from_io(err: &io::Error) -> TagWriteError {
        match err.kind() {
            io::ErrorKind::Unsupported => TagWriteError::Unsupported,
            io::ErrorKind::NotFound => TagWriteError::NotFound,
            io::ErrorKind::PermissionDenied => TagWriteError::NoPermission,
            kind @ (io::ErrorKind::Other | _) => TagWriteError::Other(kind),
        }
    }
}
