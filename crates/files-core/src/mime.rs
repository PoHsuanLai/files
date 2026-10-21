//! A file's type, as shared-mime-info names it.

use std::fmt;

use crate::MimeError;

/// A MIME type in lower case, `type/subtype`: `application/pdf`, `inode/directory`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct MimeType(String);

impl MimeType {
    /// A folder's type.
    pub const FOLDER: &'static str = "inode/directory";

    /// `text`, lower-cased, when it is one `type/subtype` pair with no spaces.
    pub fn new(text: &str) -> Result<MimeType, MimeError> {
        let mut parts = text.split('/');
        let shaped = matches!(
            (parts.next(), parts.next(), parts.next()),
            (Some(major), Some(minor), None)
                if !major.is_empty() && !minor.is_empty()
        ) && !text.contains(char::is_whitespace);
        if shaped {
            Ok(MimeType(text.to_ascii_lowercase()))
        } else {
            Err(MimeError::Shape)
        }
    }

    /// The type as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }

    /// The part before the slash: `image` of `image/png`.
    pub fn major(&self) -> &str {
        self.0.split('/').next().unwrap_or_default()
    }
}

impl fmt::Display for MimeType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
