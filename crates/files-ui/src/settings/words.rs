//! The words the settings are stored as. Each is a closed `Word` enum, so the schema names every
//! value and the Settings app picks its control by them: a pair one of whose words is `off` or
//! `hide` is a switch.

use ds::prelude::Word;
use serde::{Deserialize, Serialize};

/// `general.new_windows_show`: the folder a new window opens on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum NewWindowsShow {
    /// The home folder.
    #[default]
    Home,
    /// The Desktop folder.
    Desktop,
    /// The Documents folder.
    Documents,
    /// The Downloads folder.
    Downloads,
    /// The folder `general.new_windows_folder` names.
    #[word(label = "A folder I choose")]
    Folder,
}

/// A switch: a warning, or sorting folders on top, or tidying the Trash.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Switch {
    /// On.
    #[default]
    On,
    /// Off.
    Off,
}

impl Switch {
    /// Whether the switch is on.
    pub fn is_on(self) -> bool {
        self == Switch::On
    }
}

/// A row of the sidebar.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize, Word)]
#[serde(rename_all = "snake_case")]
#[word(case = snake)]
pub enum Row {
    /// Listed.
    #[default]
    Show,
    /// Left out.
    Hide,
}

impl Row {
    /// Whether the row is listed.
    pub fn is_shown(self) -> bool {
        self == Row::Show
    }
}
