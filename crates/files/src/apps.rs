//! The installed apps, for the window's Open With: the desktop crate's launcher behind the
//! window's [`Apps`] seam.

use std::path::PathBuf;
use std::sync::Arc;

use files_desktop::{AppOffer, DesktopLauncher};
use files_model::AppPick;
use files_ui::{AppChoiceView, Apps};

/// Answers Open With from the launcher's entries and associations.
pub struct InstalledApps {
    launcher: Arc<DesktopLauncher>,
}

impl std::fmt::Debug for InstalledApps {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("InstalledApps")
    }
}

impl InstalledApps {
    /// The apps `launcher` knows.
    pub fn new(launcher: Arc<DesktopLauncher>) -> InstalledApps {
        InstalledApps { launcher }
    }
}

/// An offer as the window shows it.
fn view_of(offer: AppOffer) -> AppChoiceView {
    AppChoiceView {
        pick: AppPick::new(offer.id.as_str()),
        name: offer.name,
        is_default: offer.is_default,
        icon: offer.icon,
    }
}

impl Apps for InstalledApps {
    fn apps_for(&self, targets: &[PathBuf]) -> Vec<AppChoiceView> {
        self.launcher
            .apps_for(targets)
            .into_iter()
            .map(view_of)
            .collect()
    }

    fn all_apps(&self) -> Vec<AppChoiceView> {
        self.launcher.all_apps().into_iter().map(view_of).collect()
    }
}
