//! Handing files to the apps that open them.

use std::path::PathBuf;

use files_model::AppPick;

use crate::error::OpenFailure;

/// Which app opens the files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum OpenAs {
    /// Each file in the app the person's settings name for its type.
    Default,
    /// All of them in this app, which the person picked.
    App(AppPick),
}

/// Opens files the way the desktop does.
///
/// This crate never starts a program; the binary gives it one that does (the desktop crate's
/// launcher, with an activation token).
pub trait Opener: Send + Sync {
    /// Opens `targets` as `with` says. A failure comes back in words a person reads and the
    /// window tells it as a toast.
    fn open(&self, targets: Vec<PathBuf>, with: OpenAs) -> Result<(), OpenFailure>;
}
