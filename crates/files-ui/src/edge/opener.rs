//! Handing files to the apps that open them.

use std::path::PathBuf;

use files_core::OnlineUrl;
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

    /// Opens Accounts in Settings, for a cloud drive that needs signing in. An opener that cannot
    /// says so in words.
    fn open_accounts(&self) -> Result<(), OpenFailure> {
        Err(OpenFailure(
            "Accounts can\u{2019}t be opened from here. Open Settings and choose Accounts."
                .to_owned(),
        ))
    }

    /// Opens an online document's address in the person's browser. An opener that cannot says
    /// so in words.
    fn open_url(&self, _url: &OnlineUrl) -> Result<(), OpenFailure> {
        Err(OpenFailure(
            "Online documents can\u{2019}t be opened from here.".to_owned(),
        ))
    }
}
