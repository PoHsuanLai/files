//! Opening files with the person's default apps: the desktop crate's launcher behind the
//! window's [`Opener`] seam.

use std::path::PathBuf;
use std::sync::Arc;

use files_desktop::{AppChoice, Launcher};
use files_ui::{OpenFailure, Opener};

/// Hands files to the launcher, each to its default app.
///
/// No activation token goes along yet: the window has no way to ask the compositor for one, so
/// the app that opens raises itself as its own policy allows. When quire offers one, it is
/// passed here.
pub struct DesktopOpener {
    launcher: Arc<dyn Launcher>,
}

impl std::fmt::Debug for DesktopOpener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopOpener")
    }
}

impl DesktopOpener {
    /// An opener over `launcher`.
    pub fn new(launcher: Arc<dyn Launcher>) -> DesktopOpener {
        DesktopOpener { launcher }
    }
}

impl Opener for DesktopOpener {
    fn open(&self, targets: Vec<PathBuf>) -> Result<(), OpenFailure> {
        self.launcher
            .open(&targets, AppChoice::Default, None)
            .map_err(|error| OpenFailure(error.to_string()))
    }
}
