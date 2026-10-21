//! The errors of this crate.

/// Why a text is not a desktop id.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum DesktopIdError {
    /// It does not end in `.desktop`.
    #[error("An app id ends in \u{201c}.desktop\u{201d}.")]
    Suffix,
    /// Nothing comes before `.desktop`.
    #[error("An app id needs a name before \u{201c}.desktop\u{201d}.")]
    Empty,
    /// It holds a `/`, or white space.
    #[error("An app id has no slash or spaces.")]
    Shape,
}

/// Why the file type database could not be loaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LoadError {
    /// No data folder held any file name patterns.
    #[error("No file type database was found.")]
    NoDatabase,
}
