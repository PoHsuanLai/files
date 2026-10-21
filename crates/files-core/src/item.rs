//! One item of a folder, named by where it is.

use std::path::{Path, PathBuf};

use crate::{FileName, FolderPath, NameError, PathError};

/// An item (file, folder or link) by the folder it is in and its name there.
///
/// Two items with the same name in different folders are different items; this is what plans of
/// work and the trash speak of, where a bare name would not say which.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub struct ItemRef {
    /// The folder it is in.
    pub folder: FolderPath,
    /// Its name there.
    pub name: FileName,
}

impl ItemRef {
    /// The item `name` in `folder`.
    pub fn new(folder: FolderPath, name: FileName) -> ItemRef {
        ItemRef { folder, name }
    }

    /// The item an absolute `path` leads to (the path as given, links not resolved).
    pub fn from_path(path: &Path) -> Result<ItemRef, ItemError> {
        let name = path.file_name().ok_or(ItemError::NoName)?;
        let parent = path.parent().ok_or(ItemError::NoName)?;
        Ok(ItemRef {
            folder: FolderPath::new(parent)?,
            name: FileName::new(name)?,
        })
    }

    /// Its full path.
    pub fn path(&self) -> PathBuf {
        self.folder.as_path().join(self.name.as_os_str())
    }

    /// The same name in another folder.
    pub fn moved_to(&self, folder: &FolderPath) -> ItemRef {
        ItemRef {
            folder: folder.clone(),
            name: self.name.clone(),
        }
    }

    /// The same folder, another name.
    pub fn renamed(&self, name: FileName) -> ItemRef {
        ItemRef {
            folder: self.folder.clone(),
            name,
        }
    }

    /// The item as a folder to look inside.
    pub fn as_folder(&self) -> FolderPath {
        self.folder.join(&self.name)
    }
}
