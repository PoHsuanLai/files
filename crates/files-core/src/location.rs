//! Where a window looks.

use std::path::{Path, PathBuf};

use crate::{PathError, TagColour};

/// A place a window shows. Grows with the app (Recents, searches, volumes), one variant per
/// place that is built.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Location {
    /// A folder on disk.
    Folder(FolderPath),
    /// The Trash: every trashed item of every volume.
    Trash,
    /// Every file under the home folder that carries this colour's tag: a search, listed as it is
    /// found.
    Tag(TagColour),
}

/// An absolute path to a folder, as the person reached it (links not resolved, so the path bar
/// shows the way they came).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct FolderPath(PathBuf);

impl FolderPath {
    /// `path`, when it is absolute.
    pub fn new(path: impl Into<PathBuf>) -> Result<FolderPath, PathError> {
        let path = path.into();
        if path.is_absolute() {
            Ok(FolderPath(path))
        } else {
            Err(PathError::Relative)
        }
    }

    /// The path.
    pub fn as_path(&self) -> &Path {
        &self.0
    }

    /// The folder above, unless this is `/`.
    pub fn parent(&self) -> Option<FolderPath> {
        self.0.parent().map(|p| FolderPath(p.to_path_buf()))
    }

    /// The child folder `name` inside this one.
    pub fn join(&self, name: &crate::FileName) -> FolderPath {
        FolderPath(self.0.join(name.as_os_str()))
    }
}
