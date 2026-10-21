//! Handing files to the apps that open them.

use std::path::PathBuf;

/// Opens files the way the desktop does: each in the app the person's settings name for its type.
///
/// This crate never starts a program; the binary (and later the desktop crate, with an activation
/// token) gives it one that does.
pub trait Opener: Send + Sync {
    /// Opens each of `targets` in its app. Failures are the opener's to report; the window has
    /// nothing to do about them.
    fn open(&self, targets: Vec<PathBuf>);
}
