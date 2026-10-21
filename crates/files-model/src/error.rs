//! Why a folder could not be read, a typed path not gone to, or a name not used, in the words a
//! person reads.

use files_core::NameError;

/// Why a folder could not be read, in terms a person understands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ListFailure {
    /// It isn't there any more.
    #[error("That folder can't be found.")]
    NotFound,
    /// You're not allowed to look inside.
    #[error("You don't have permission to open that folder.")]
    NoPermission,
    /// It's a file, not a folder.
    #[error("That isn't a folder.")]
    NotAFolder,
    /// Anything else.
    #[error("That folder can't be opened.")]
    Other,
}

/// Why a typed path can't be gone to, in the words under the field.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum GoToProblem {
    /// It isn't a folder that exists (or isn't a full path at all).
    #[error("The folder can't be found.")]
    NotFound,
    /// It exists, and the person may not look inside.
    #[error("You don't have permission to open that folder.")]
    NoPermission,
}

/// Why a rename can't go through yet, in words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RenameProblem {
    /// The text isn't a name.
    #[error("{0}")]
    Invalid(NameError),
    /// Something in the folder already has that name.
    #[error("That name is taken.")]
    Taken,
}
