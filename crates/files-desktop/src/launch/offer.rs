//! Which apps can open some files, for an Open With menu.

use std::path::PathBuf;

use ds_helpers::DesktopId;

use super::desktop::DesktopLauncher;

/// One app offered to open files with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppOffer {
    /// Its desktop file id; what [`AppChoice::App`](super::AppChoice::App) takes.
    pub id: DesktopId,
    /// Its name, from its desktop entry.
    pub name: String,
    /// Whether it is the app the person's settings name for the first file's type.
    pub is_default: bool,
    /// The image file of its icon, when its entry names one and the icon theme has it.
    pub icon: Option<PathBuf>,
}

impl DesktopLauncher {
    /// The installed apps that open every one of `targets`: those of the first file's type that
    /// the other files' types have too, its default first. Nothing for no targets.
    pub fn apps_for(&self, targets: &[PathBuf]) -> Vec<AppOffer> {
        let kinds: Vec<_> = targets
            .iter()
            .filter_map(|target| self.kind_of(target))
            .collect();
        let Some((first, rest)) = kinds.split_first() else {
            return Vec::new();
        };
        let default = self.associations.default_for(first, &self.db);
        let others: Vec<Vec<DesktopId>> = rest
            .iter()
            .map(|kind| self.associations.apps_for(kind, &self.db))
            .collect();
        self.associations
            .apps_for(first, &self.db)
            .into_iter()
            .filter(|id| others.iter().all(|apps| apps.contains(id)))
            .filter_map(|id| {
                let app = self.app(&id)?;
                let is_default = default.as_ref() == Some(&id);
                Some(AppOffer {
                    name: app.name.clone(),
                    icon: self.icons.resolve(app.icon.as_deref()),
                    id,
                    is_default,
                })
            })
            .collect()
    }

    /// Every installed app, in the order they were found, none of them the default.
    pub fn all_apps(&self) -> Vec<AppOffer> {
        self.apps
            .iter()
            .map(|app| AppOffer {
                id: app.id.clone(),
                name: app.name.clone(),
                is_default: false,
                icon: self.icons.resolve(app.icon.as_deref()),
            })
            .collect()
    }
}
