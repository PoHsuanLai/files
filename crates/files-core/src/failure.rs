//! Why a piece of work did not go through, in the words a person reads.

use std::io;

/// Why a change to the files failed.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum OpFailure {
    /// An item is gone.
    #[error("An item can't be found any more.")]
    NotFound,
    /// The system won't let this person change it.
    #[error("You don't have permission to change that.")]
    NoPermission,
    /// Another item already has the name.
    #[error("That name is taken.")]
    Taken,
    /// The disk is full.
    #[error("There is not enough room on the disk.")]
    NoSpace,
    /// A folder was to be put inside itself.
    #[error("A folder can't be put inside itself.")]
    IntoItself,
    /// A name would be too long.
    #[error("That name is too long.")]
    NameTooLong,
    /// Something is in the place an item is to be put back to.
    #[error("Something is already in the place it came from.")]
    Occupied,
    /// The Trash can't be used.
    #[error("The Trash can't be used right now.")]
    TrashUnavailable,
    /// Anything else the system reported.
    #[error("That couldn't be done.")]
    Other,
}

impl OpFailure {
    /// The failure an I/O error stands for.
    pub fn from_io(err: &io::Error) -> OpFailure {
        match err.kind() {
            io::ErrorKind::NotFound => OpFailure::NotFound,
            io::ErrorKind::PermissionDenied => OpFailure::NoPermission,
            io::ErrorKind::AlreadyExists => OpFailure::Taken,
            io::ErrorKind::StorageFull => OpFailure::NoSpace,
            io::ErrorKind::Other | _ => OpFailure::Other,
        }
    }
}
