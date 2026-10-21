//! The errors of this crate.

/// Why files were not opened, in the words the window shows (a toast).
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{0}")]
pub struct OpenFailure(pub String);
