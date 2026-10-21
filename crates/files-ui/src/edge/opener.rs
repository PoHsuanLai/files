//! Handing files to the apps that open them.

use std::path::PathBuf;

use crate::error::OpenFailure;

/// Opens files the way the desktop does: each in the app the person's settings name for its type.
///
/// This crate never starts a program; the binary gives it one that does (the desktop crate's
/// launcher, with an activation token).
pub trait Opener: Send + Sync {
    /// Opens each of `targets` in its app. A failure comes back in words a person reads and the
    /// window tells it as a toast.
    fn open(&self, targets: Vec<PathBuf>) -> Result<(), OpenFailure>;
}
