//! Building the launcher from the person's installed apps and file associations.

use std::path::PathBuf;
use std::sync::Arc;

use files_desktop::{
    Activate, ActivationToken, DesktopLauncher, StartError, SystemSpawn, installed_check, load_apps,
};
use files_mime::{Associations, MimeDb, XdgDirs};

/// The XDG folders, read once from the environment (`$XDG_DATA_HOME` and friends, with the
/// specification's defaults).
pub fn xdg_dirs() -> XdgDirs {
    let home = dirs::home_dir().unwrap_or_default();
    XdgDirs {
        data_home: dirs::data_dir().unwrap_or_else(|| home.join(".local/share")),
        data_dirs: listed("XDG_DATA_DIRS", &["/usr/local/share", "/usr/share"]),
        config_home: dirs::config_dir().unwrap_or_else(|| home.join(".config")),
        config_dirs: listed("XDG_CONFIG_DIRS", &["/etc/xdg"]),
        current_desktops: std::env::var("XDG_CURRENT_DESKTOP")
            .map(|value| {
                value
                    .split(':')
                    .filter(|part| !part.is_empty())
                    .map(str::to_owned)
                    .collect()
            })
            .unwrap_or_default(),
    }
}

/// The folders of a colon-separated variable, or `default` when it is unset or empty.
fn listed(variable: &str, default: &[&str]) -> Vec<PathBuf> {
    let from_env: Vec<PathBuf> = std::env::var_os(variable)
        .map(|value| std::env::split_paths(&value).collect())
        .unwrap_or_default();
    if from_env.is_empty() {
        default.iter().map(PathBuf::from).collect()
    } else {
        from_env
    }
}

/// The launcher over what is installed. With no file type database every file is of an unknown
/// type, so the window says there is no app for it.
pub fn launcher(dirs: &XdgDirs, activate: Arc<dyn Activate>) -> DesktopLauncher {
    let db = MimeDb::load(dirs).map_or_else(|_| MimeDb::from_layers(&[]), |loaded| loaded.value);
    let applications: Vec<PathBuf> = std::iter::once(&dirs.data_home)
        .chain(dirs.data_dirs.iter())
        .map(|dir| dir.join("applications"))
        .collect();
    let apps = load_apps(&applications);
    let associations = Associations::load(dirs, installed_check(&apps));
    DesktopLauncher::new(db, associations, apps, Arc::new(SystemSpawn), activate)
}

/// Starting apps over the bus when there is no bus to ask: every such app fails in words.
#[derive(Debug, Clone, Copy)]
pub struct NoBus;

impl Activate for NoBus {
    fn open(
        &self,
        _bus_name: &str,
        _uris: &[String],
        _token: Option<&ActivationToken>,
    ) -> Result<(), StartError> {
        Err(StartError::new("The desktop isn\u{2019}t answering."))
    }
}
