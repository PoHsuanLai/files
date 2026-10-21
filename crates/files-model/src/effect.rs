//! What the model asks its caller to do.

use std::path::PathBuf;

use files_core::{FileName, FolderPath, FolderPrefs, Location};

/// One file to hand to an app: where it is, and what it is called.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct OpenTarget {
    /// The folder it is in.
    pub folder: FolderPath,
    /// Its name there.
    pub name: FileName,
}

impl OpenTarget {
    /// The file called `name` in `folder`.
    pub fn new(folder: FolderPath, name: FileName) -> OpenTarget {
        OpenTarget { folder, name }
    }

    /// The full path of the file.
    pub fn path(&self) -> PathBuf {
        self.folder.as_path().join(self.name.as_os_str())
    }
}

/// Something for the caller to carry out; the answer, if any, comes back as an
/// [`Input`](crate::Input).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Effect {
    /// Read the folder and answer with `Input::Listed`.
    List(Location),
    /// Start telling us about changes there, as `Input::Changed`.
    Watch(Location),
    /// Stop telling us about changes there.
    Unwatch(Location),
    /// Hand these files to the apps that open them.
    OpenFiles(Vec<OpenTarget>),
    /// Rename a file in a folder; the listing changes when the disk does.
    Rename {
        /// The folder it is in.
        folder: FolderPath,
        /// Its name now.
        from: FileName,
        /// The name it should have.
        to: FileName,
    },
    /// Look up how this folder was last shown and answer with `Input::PrefsLoaded`.
    LoadPrefs(Location),
    /// Remember how this folder is shown.
    SavePrefs(Location, FolderPrefs),
}
