//! What can go wrong with the Trash, in the words a person reads.

use trash::Error as Foreign;

/// Why the Trash could not do what was asked.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TrashError {
    /// An item is gone: it was not where it was said to be.
    #[error("An item can't be found any more.")]
    NotFound,
    /// The system won't let this person do it.
    #[error("You don't have permission to do that.")]
    NoPermission,
    /// Something is already in the place an item is to be put back to.
    #[error("Something is already in the place it came from.")]
    Occupied,
    /// Anything else the system reported.
    #[error("The Trash can't be used right now.")]
    Other,
}

impl TrashError {
    /// The error `err` stands for.
    pub(crate) fn of_foreign(err: Foreign) -> TrashError {
        match err {
            Foreign::FileSystem { source, .. } => TrashError::of_io(&source),
            Foreign::CouldNotAccess { .. } | Foreign::CanonicalizePath { .. } => {
                TrashError::NotFound
            }
            Foreign::RestoreCollision { .. } | Foreign::RestoreTwins { .. } => TrashError::Occupied,
            Foreign::Unknown { .. }
            | Foreign::Os { .. }
            | Foreign::TargetedRoot
            | Foreign::ConvertOsString { .. } => TrashError::Other,
        }
    }

    /// The error an I/O failure stands for.
    pub(crate) fn of_io(err: &std::io::Error) -> TrashError {
        match err.kind() {
            std::io::ErrorKind::NotFound => TrashError::NotFound,
            std::io::ErrorKind::PermissionDenied => TrashError::NoPermission,
            std::io::ErrorKind::AlreadyExists => TrashError::Occupied,
            std::io::ErrorKind::Other | _ => TrashError::Other,
        }
    }
}
