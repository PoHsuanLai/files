//! files' own settings: one file, `files/settings.toml`, and the schema that describes it
//! (quire design/22 section 9).
//!
//! Each table is a struct deriving `SettingsSchema`, so the desktop's Settings app draws a Files
//! page from the schema `files --write-schema <dir>` writes, and writes the same file this reads.
//! `ds_settings::Store` reads it leniently (a bad value costs only its key), and [`follow`] watches
//! it, so a change made in the Settings app reaches an open window.
//!
//! What is not a choice stays where it was: how each folder is shown is state (`folders.toml` in
//! the state folder), and the look is the desktop's (`quire/appearance.toml`).

mod apply;
mod follow;
mod sections;
mod words;

use std::io;
use std::path::{Path, PathBuf};

use ds_settings::schema::{AppId, FilePath, KeySpec, Schema, SettingsSchema, WordLabels};
use ds_settings::{AppName, ConfigRoot, FileName, Format, SettingsDoc, Store};
use serde::{Deserialize, Serialize};

use ds::prelude::Word;

pub use apply::NewWindow;
pub use follow::follow;
pub use sections::{AdvancedSettings, GeneralSettings, SidebarSettings};
pub use words::{NewWindowsShow, Row, Switch};

/// The directory under the config root, and the schema's app id.
pub const APP: AppName = AppName("files");

/// The settings file, relative to the config root, as the schema names it.
const FILE: &str = "files/settings.toml";

/// The `version` the file is written with. A change that is not backward compatible bumps it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct Version(u32);

impl Default for Version {
    fn default() -> Version {
        Version(1)
    }
}

/// `files/settings.toml`, whole.
#[derive(Debug, Clone, PartialEq, Eq, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct FilesDoc {
    /// Which defaults the file was written under.
    pub version: Version,
    /// `general.*`.
    pub general: GeneralSettings,
    /// `sidebar.*`.
    pub sidebar: SidebarSettings,
    /// `advanced.*`.
    pub advanced: AdvancedSettings,
}

impl SettingsDoc for FilesDoc {
    const FILE: FileName = FileName("settings.toml");
    const FORMAT: Format = Format::Toml;
}

/// The store `settings.toml` lives in, under `root`.
pub fn store(root: ConfigRoot) -> Store {
    Store::new(root, APP)
}

/// The settings in force under `root`: the defaults for a file that is missing, and the default
/// for any one value that cannot be used.
pub fn load(root: &ConfigRoot) -> FilesDoc {
    store(root.clone()).load::<FilesDoc>().value
}

/// The one `files.settings.toml` the package installs: every table's keys under app `files`.
pub fn schema() -> Schema {
    let key = [
        GeneralSettings::schema(),
        SidebarSettings::schema(),
        AdvancedSettings::schema(),
    ]
    .into_iter()
    .flat_map(|schema| schema.key)
    .map(with_word_labels)
    .collect();
    Schema {
        app: AppId(APP.0.to_owned()),
        file: FilePath(FILE.to_owned()),
        version: 1,
        key,
        // The file holds no tables of another program's.
        foreign: Default::default(),
        footnotes: Vec::new(),
        // Every key sits on files' own page; none is declared.
        pages: Vec::new(),
    }
}

/// `key` with the words a person reads for its values, where they are not the stored word written
/// out: "A folder I choose" for `folder`.
fn with_word_labels(mut key: KeySpec) -> KeySpec {
    if key.path.0 == "general.new_windows_show" {
        key.labels = WordLabels(
            NewWindowsShow::ALL
                .iter()
                .map(|word| (word.slug().to_owned(), word.label().to_owned()))
                .collect(),
        );
    }
    key
}

/// `files --write-schema <dir>`: writes `files.settings.toml` into `dir` and says where it went.
pub fn write_schema(dir: &Path) -> io::Result<PathBuf> {
    schema().write_to(dir)
}
