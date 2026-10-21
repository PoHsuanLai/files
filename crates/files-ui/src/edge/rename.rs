//! Renaming a file on disk.

use std::io;

use files_core::{FileName, FolderPath};

/// Why a rename did not happen, in the words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RenameError {
    /// The item is gone.
    #[error("That item can't be found any more.")]
    NotFound,
    /// You may not change it.
    #[error("You don't have permission to rename that item.")]
    NoPermission,
    /// Another item already has the name.
    #[error("That name is taken.")]
    Taken,
    /// Anything else the system reported.
    #[error("That item can't be renamed.")]
    Other,
}

impl RenameError {
    /// The error an I/O failure stands for.
    pub fn from_io(err: &io::Error) -> RenameError {
        match err.kind() {
            io::ErrorKind::NotFound => RenameError::NotFound,
            io::ErrorKind::PermissionDenied => RenameError::NoPermission,
            io::ErrorKind::AlreadyExists => RenameError::Taken,
            io::ErrorKind::Other | _ => RenameError::Other,
        }
    }
}

/// Renames files. Real: [`DiskRenamer`]; a test gives its own.
pub trait Renamer: Send + Sync {
    /// Renames `from` to `to` inside `folder`, and never replaces an item that has the name.
    fn rename(
        &self,
        folder: &FolderPath,
        from: &FileName,
        to: &FileName,
    ) -> Result<(), RenameError>;
}

/// The real disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskRenamer;

impl Renamer for DiskRenamer {
    fn rename(
        &self,
        folder: &FolderPath,
        from: &FileName,
        to: &FileName,
    ) -> Result<(), RenameError> {
        let (from, to) = (
            folder.as_path().join(from.as_os_str()),
            folder.as_path().join(to.as_os_str()),
        );
        // `rename(2)` replaces what is at the target; a rename never should. The window checked
        // the names it knew, so this only catches a file that appeared since.
        if std::fs::symlink_metadata(&to).is_ok() {
            return Err(RenameError::Taken);
        }
        std::fs::rename(from, to).map_err(|err| RenameError::from_io(&err))
    }
}
