//! Turning what the system says into `files_core::OpFailure`, the one error of a change.

use std::io;

use files_core::OpFailure;
use files_trash::TrashError;

/// The failure an I/O error stands for; for `map_err`.
pub(crate) fn io(err: io::Error) -> OpFailure {
    OpFailure::from_io(&err)
}

/// The failure a Trash error stands for; for `map_err`.
pub(crate) fn trash(err: TrashError) -> OpFailure {
    match err {
        TrashError::NotFound => OpFailure::NotFound,
        TrashError::NoPermission => OpFailure::NoPermission,
        TrashError::Occupied => OpFailure::Occupied,
        TrashError::Other | _ => OpFailure::TrashUnavailable,
    }
}
