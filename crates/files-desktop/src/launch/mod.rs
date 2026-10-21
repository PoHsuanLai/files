//! Opening files with apps, as the Desktop Entry Specification says to.

mod activate;
mod desktop;
mod entry;
mod exec;
mod spawn;

pub use activate::{Activate, BusActivate};
pub use desktop::DesktopLauncher;
pub use entry::{AppEntry, EntryError, Start, Terminal, installed_check, load_apps};
pub use exec::{ExecError, ExecLine, Expand, Takes};
pub use spawn::{Spawn, SpawnCommand, StartError, SystemSpawn};

#[cfg(feature = "testing")]
pub use activate::{ActivateCall, FakeActivate};
#[cfg(feature = "testing")]
pub use spawn::FakeSpawn;

use std::path::PathBuf;

use files_mime::DesktopId;

use crate::token::ActivationToken;

/// Which app opens the files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppChoice {
    /// Each file's own default app.
    Default,
    /// This app, for all of them.
    App(DesktopId),
}

/// Why files were not opened, in the words a person reads.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LaunchError {
    /// Nothing opens this file. Holds the file's name.
    #[error("There is no app set to open \u{201c}{0}\u{201d}.")]
    NoApp(String),
    /// The app runs in a terminal window, which files cannot open yet. Holds the app's name.
    #[error("\u{201c}{0}\u{201d} runs in a terminal window, which Files can\u{2019}t open yet.")]
    NeedsTerminal(String),
    /// The app could not be started.
    #[error("\u{201c}{app}\u{201d} couldn\u{2019}t be opened. {reason}")]
    Failed {
        /// The app's name.
        app: String,
        /// What went wrong.
        reason: String,
    },
}

/// Opens files with apps.
pub trait Launcher: Send + Sync {
    /// Opens `targets` with `with`. `token` lets the app raise its window.
    ///
    /// Every app that can be started is; the first failure is returned after the others ran.
    fn open(
        &self,
        targets: &[PathBuf],
        with: AppChoice,
        token: Option<ActivationToken>,
    ) -> Result<(), LaunchError>;
}
