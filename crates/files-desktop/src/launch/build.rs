//! Building a [`DesktopLauncher`] from what it needs and what it may be given.

use std::sync::Arc;

use files_mime::{Associations, MimeDb};

use super::activate::Activate;
use super::desktop::DesktopLauncher;
use super::entry::AppEntry;
use super::icons::{AppIcons, IconLookup, NoIcons};
use super::spawn::{Spawn, SystemSpawn};

/// What the installed apps and the person's settings say: the type database, the associations
/// and the apps' entries.
#[derive(Debug)]
pub struct Catalog {
    /// The shared type database.
    pub db: MimeDb,
    /// Which apps open which types.
    pub associations: Associations,
    /// The installed apps.
    pub apps: Vec<AppEntry>,
}

/// A [`DesktopLauncher`] being built. Apps are started with [`SystemSpawn`] and have no icons
/// unless said otherwise.
pub struct LauncherBuilder {
    catalog: Catalog,
    activate: Arc<dyn Activate>,
    spawn: Arc<dyn Spawn>,
    icons: Arc<dyn IconLookup>,
}

impl std::fmt::Debug for LauncherBuilder {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("LauncherBuilder")
            .field("apps", &self.catalog.apps.len())
            .finish_non_exhaustive()
    }
}

impl LauncherBuilder {
    pub(super) fn new(catalog: Catalog, activate: Arc<dyn Activate>) -> LauncherBuilder {
        LauncherBuilder {
            catalog,
            activate,
            spawn: Arc::new(SystemSpawn),
            icons: Arc::new(NoIcons),
        }
    }

    /// Starts commands with `spawn` instead of real processes.
    pub fn spawn(mut self, spawn: Arc<dyn Spawn>) -> LauncherBuilder {
        self.spawn = spawn;
        self
    }

    /// Finds icons by name with `icons`.
    pub fn icons(mut self, icons: Arc<dyn IconLookup>) -> LauncherBuilder {
        self.icons = icons;
        self
    }

    /// The launcher.
    pub fn build(self) -> DesktopLauncher {
        let Catalog {
            db,
            associations,
            apps,
        } = self.catalog;
        DesktopLauncher {
            db,
            associations,
            apps,
            spawn: self.spawn,
            activate: self.activate,
            icons: AppIcons::new(self.icons),
        }
    }
}
