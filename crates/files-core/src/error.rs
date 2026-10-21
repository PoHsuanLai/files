//! Why a name, a path, an item, a tag or a type is refused, in the words a person reads.

/// Why a path is not an item.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ItemError {
    /// Not an absolute path.
    #[error("That is not a full path.")]
    Path(#[from] PathError),
    /// The last part is not a usable name.
    #[error("{0}")]
    Name(#[from] NameError),
    /// The path has no last part (`/`, or one that ends in `..`).
    #[error("That path names no item.")]
    NoName,
}

/// Why a name is not a tag, in the words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum TagError {
    /// Nothing was typed.
    #[error("A tag can't be empty.")]
    Empty,
    /// A comma, which separates tags.
    #[error("A tag can't contain a comma.")]
    Comma,
    /// A control character, such as a line break.
    #[error("A tag can't contain a line break or other control character.")]
    Control,
}

/// Why a path is not a folder location.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum PathError {
    /// A relative path, which means nothing without a current folder.
    #[error("not an absolute path")]
    Relative,
}

/// Why a name was refused, in the words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum NameError {
    /// Nothing was typed.
    #[error("A name can't be empty.")]
    Empty,
    /// `.` or `..`, which name this folder and the one above it.
    #[error("The names \u{201c}.\u{201d} and \u{201c}..\u{201d} are reserved.")]
    Reserved,
    /// A `/`, which separates folders in a path.
    #[error("A name can't contain \u{201c}/\u{201d}.")]
    Slash,
    /// A NUL character, which no Linux name may hold.
    #[error("A name can't contain a null character.")]
    Nul,
    /// Over 255 bytes.
    #[error("That name is too long.")]
    TooLong,
}

/// Why a text is not a MIME type.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum MimeError {
    /// It is not one `type/subtype` pair of non-empty parts.
    #[error("not a type/subtype pair")]
    Shape,
}
