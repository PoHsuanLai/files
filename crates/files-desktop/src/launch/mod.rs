//! Opening files with apps, as the Desktop Entry Specification says to.

mod activate;
mod build;
mod desktop;
mod entry;
mod error;
mod exec;
mod icons;
mod offer;
mod spawn;

pub use activate::{Activate, BusActivate};
pub use build::{Catalog, LauncherBuilder};
pub use desktop::DesktopLauncher;
pub use entry::{AppEntry, Start, Terminal, installed_check, load_apps};
pub use error::{EntryError, ExecError, LaunchError, StartError};
pub use exec::{ExecLine, Expand, Takes};
pub use icons::{AppIcons, IconLookup, NoIcons};
pub use offer::{AppOffer, Standing};
pub use spawn::{Spawn, SpawnCommand, SystemSpawn};

#[cfg(feature = "testing")]
pub use activate::{ActivateCall, FakeActivate};
#[cfg(feature = "testing")]
pub use icons::FakeIcons;
#[cfg(feature = "testing")]
pub use spawn::FakeSpawn;

use std::path::PathBuf;

use ds_helpers::DesktopId;

use crate::token::ActivationToken;

/// Which app opens the files.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppChoice {
    /// Each file's own default app.
    Default,
    /// This app, for all of them.
    App(DesktopId),
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
