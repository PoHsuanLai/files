//! The errors of this crate.

/// Why files were not opened, in the words the window shows (a toast).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct OpenFailure(pub String);

/// Why the disks could not be listed or watched. The window shows none; it tells nobody.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum VolumeError {
    /// This computer has no service that lists disks, or it did not answer.
    #[error("The disks can't be listed right now.")]
    Unavailable,
}

/// Why the window could not answer a question.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ReadError {
    /// The window closed before it answered.
    #[error("Files was closed before it could answer.")]
    Closed,
}
