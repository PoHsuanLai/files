//! How files meets the desktop: the file manager service other programs call, one running
//! instance, and opening files with apps. This is the only crate of the app that names the session
//! bus.
//!
//! Three parts, each usable on its own:
//!
//! - [`file_manager`]: `org.freedesktop.FileManager1`, parsed into typed [`Show`] requests that go
//!   to a sink the app gives;
//! - [`instance`]: [`claim`] the names, or [`forward`] a request to the app that has them;
//! - [`launch`]: open files with the default app or a chosen one, per the Desktop Entry
//!   Specification, behind the [`Launcher`] trait.
//!
//! No part reads the environment: the caller passes the bus connection, the folders and the
//! tokens in.
//!
//! ```
//! use files_desktop::{ExecLine, Expand, Takes};
//! use std::path::{Path, PathBuf};
//!
//! let line = ExecLine::parse(r#""/opt/My App/run" --open %F"#)?;
//! assert_eq!(line.takes(), Takes::ManyFiles);
//! let files = [PathBuf::from("/a/1.txt"), PathBuf::from("/a/2.txt")];
//! let argv = line.expand(&Expand {
//!     files: &files,
//!     icon: None,
//!     name: "My App",
//!     entry_path: Path::new("/usr/share/applications/app.desktop"),
//! });
//! assert_eq!(argv.len(), 4);
//! assert_eq!(argv[0], "/opt/My App/run");
//! # Ok::<(), files_desktop::ExecError>(())
//! ```

pub mod file_manager;
pub mod instance;
pub mod launch;
mod token;
pub mod uri;

pub use file_manager::{Reveal, Show, ShowKind, ShowRequest, Sink, StartupId};
pub use instance::{Bus, Claim, FileManagerName, InstanceError, Served, claim, forward, session};
pub use launch::{
    Activate, AppChoice, AppEntry, BusActivate, DesktopLauncher, EntryError, ExecError, ExecLine,
    Expand, LaunchError, Launcher, Spawn, SpawnCommand, StartError, SystemSpawn, Takes,
    installed_check, load_apps,
};
pub use token::ActivationToken;

#[cfg(feature = "testing")]
pub use launch::{ActivateCall, FakeActivate, FakeSpawn};
