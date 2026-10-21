//! The id of an installed app: its desktop file's name.

use std::fmt;

use crate::error::DesktopIdError;

/// A desktop file id such as `org.gnome.Nautilus.desktop`.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DesktopId(String);

impl DesktopId {
    /// `text`, when it ends in `.desktop` after a name, with no slash or white space.
    pub fn new(text: &str) -> Result<DesktopId, DesktopIdError> {
        let Some(stem) = text.strip_suffix(".desktop") else {
            return Err(DesktopIdError::Suffix);
        };
        if stem.is_empty() {
            return Err(DesktopIdError::Empty);
        }
        if text.contains('/') || text.contains(char::is_whitespace) {
            return Err(DesktopIdError::Shape);
        }
        Ok(DesktopId(text.to_owned()))
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for DesktopId {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}
