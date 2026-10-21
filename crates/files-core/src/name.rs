//! A file's own name: one path component, as Linux allows it.

use std::ffi::{OsStr, OsString};
use std::fmt;
use std::os::unix::ffi::OsStrExt;

use crate::NameError;

/// The longest name a Linux filesystem takes, in bytes (`NAME_MAX`).
const NAME_MAX: usize = 255;

/// One path component: not empty, not `.` or `..`, with no `/` and no NUL, at most 255 bytes.
///
/// Linux names are bytes, not text, so a name that is not valid UTF-8 is still a name; it is shown
/// with [`FileName::display`], which replaces what cannot be read.
#[derive(Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FileName(OsString);

impl FileName {
    /// `name`, when it is a valid file name.
    pub fn new(name: impl Into<OsString>) -> Result<FileName, NameError> {
        let name = name.into();
        let bytes = name.as_bytes();
        match bytes {
            [] => Err(NameError::Empty),
            b"." | b".." => Err(NameError::Reserved),
            _ if bytes.contains(&b'/') => Err(NameError::Slash),
            _ if bytes.contains(&0) => Err(NameError::Nul),
            _ if bytes.len() > NAME_MAX => Err(NameError::TooLong),
            _ => Ok(FileName(name)),
        }
    }

    /// The name as the filesystem holds it.
    pub fn as_os_str(&self) -> &OsStr {
        &self.0
    }

    /// The name for a person to read; bytes that are not UTF-8 become U+FFFD.
    pub fn display(&self) -> std::borrow::Cow<'_, str> {
        self.0.to_string_lossy()
    }

    /// Hidden when it starts with a dot, as every Linux file manager reads it.
    pub fn visibility(&self) -> crate::Visibility {
        match self.0.as_bytes() {
            [b'.', ..] => crate::Visibility::Hidden,
            _ => crate::Visibility::Shown,
        }
    }

    /// The name without its extension: `Report.final` of `Report.final.pdf`. A name that starts
    /// with its only dot (`.bashrc`) has no extension, so its stem is the whole name.
    pub fn stem(&self) -> &OsStr {
        self.split().0
    }

    /// The bytes after the last dot, when the name has an extension: `pdf` of `Report.pdf`.
    pub fn extension(&self) -> Option<&OsStr> {
        self.split().1
    }

    fn split(&self) -> (&OsStr, Option<&OsStr>) {
        let bytes = self.0.as_bytes();
        match bytes.iter().rposition(|b| *b == b'.') {
            Some(0) | None => (&self.0, None),
            Some(at) if at + 1 == bytes.len() => (&self.0, None),
            Some(at) => (
                OsStr::from_bytes(&bytes[..at]),
                Some(OsStr::from_bytes(&bytes[at + 1..])),
            ),
        }
    }
}

impl fmt::Debug for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_tuple("FileName").field(&self.0).finish()
    }
}

impl fmt::Display for FileName {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.display())
    }
}

impl AsRef<OsStr> for FileName {
    fn as_ref(&self) -> &OsStr {
        &self.0
    }
}
