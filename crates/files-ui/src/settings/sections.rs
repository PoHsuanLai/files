//! The tables of `files/settings.toml`. Each is a settings domain with its own schema keys, in
//! the order the Settings app lists them.

use ds_settings::schema::Page;
use serde::{Deserialize, Serialize};

use super::APP;
use super::words::{NewWindowsShow, Row, Switch};

/// The Settings page the keys are on: files' own.
fn page() -> Page {
    Page::App(APP.0.to_owned())
}

/// `general.*`: where a new window opens.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(default)]
#[settings(file = "files/settings.toml", domain = "general", page = page())]
pub struct GeneralSettings {
    /// `general.new_windows_show`.
    #[settings(
        label = "New windows show",
        help = "The folder a new window opens on when none is asked for.",
        section = "General"
    )]
    pub new_windows_show: NewWindowsShow,
    /// `general.new_windows_folder`.
    #[settings(
        label = "Chosen folder",
        help = "The path of the folder new windows open on when the choice above is a folder I choose.",
        section = "General"
    )]
    pub new_windows_folder: String,
}

/// `sidebar.*`: which rows the sidebar lists.
#[derive(
    Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize, ds_settings::SettingsSchema,
)]
#[serde(default)]
#[settings(file = "files/settings.toml", domain = "sidebar", page = page())]
pub struct SidebarSettings {
    /// `sidebar.home`.
    #[settings(label = "Home", section = "Favorites")]
    pub home: Row,
    /// `sidebar.desktop`.
    #[settings(label = "Desktop", section = "Favorites")]
    pub desktop: Row,
    /// `sidebar.documents`.
    #[settings(label = "Documents", section = "Favorites")]
    pub documents: Row,
    /// `sidebar.downloads`.
    #[settings(label = "Downloads", section = "Favorites")]
    pub downloads: Row,
    /// `sidebar.pictures`.
    #[settings(label = "Pictures", section = "Favorites")]
    pub pictures: Row,
    /// `sidebar.music`.
    #[settings(label = "Music", section = "Favorites")]
    pub music: Row,
    /// `sidebar.videos`.
    #[settings(label = "Videos", section = "Favorites")]
    pub videos: Row,
    /// `sidebar.computer`.
    #[settings(
        label = "Computer",
        help = "The whole disk, from its root.",
        section = "Locations"
    )]
    pub computer: Row,
    /// `sidebar.disks`.
    #[settings(
        label = "Disks",
        help = "The disks that are attached, with their Eject buttons.",
        section = "Locations"
    )]
    pub disks: Row,
}

/// `advanced.*`: sorting, warnings and the Trash.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, ds_settings::SettingsSchema)]
#[serde(default)]
#[settings(file = "files/settings.toml", domain = "advanced", page = page())]
pub struct AdvancedSettings {
    /// `advanced.folders_on_top`.
    #[settings(
        label = "Keep folders on top",
        help = "When sorting by name, folders come before files.",
        section = "Sorting"
    )]
    pub folders_on_top: Switch,
    /// `advanced.warn_changing_extension`.
    #[settings(
        label = "Show warning before changing an extension",
        section = "Warnings"
    )]
    pub warn_changing_extension: Switch,
    /// `advanced.warn_emptying_trash`.
    #[settings(label = "Show warning before emptying the Trash", section = "Warnings")]
    pub warn_emptying_trash: Switch,
    /// `advanced.remove_old_trash`.
    #[settings(
        label = "Remove items from the Trash after 30 days",
        help = "Done when Files starts, and when this is turned on.",
        section = "Trash"
    )]
    pub remove_old_trash: Switch,
}

impl Default for AdvancedSettings {
    /// Finder's defaults: folders mixed in, both warnings on, the Trash kept.
    fn default() -> AdvancedSettings {
        AdvancedSettings {
            folders_on_top: Switch::Off,
            warn_changing_extension: Switch::On,
            warn_emptying_trash: Switch::On,
            remove_old_trash: Switch::Off,
        }
    }
}
