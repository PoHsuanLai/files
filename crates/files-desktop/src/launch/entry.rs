//! An installed app, read from its desktop entry.

use std::collections::HashSet;
use std::path::{Path, PathBuf};

use ds_helpers::mime::Installed;
use ds_helpers::{ApplicationEntry, DESKTOP_ENTRY_CAP, DesktopFile, DesktopId};

use super::EntryError;

/// Whether the app wants a terminal window (`Terminal=true`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Terminal {
    /// It runs in a terminal.
    Needed,
    /// It has a window of its own.
    NotNeeded,
}

/// How the app is started.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Start {
    /// By running its `Exec` command.
    Command,
    /// By asking its bus name to open the files (`DBusActivatable=true`).
    Bus,
}

/// What launching needs to know about one app.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppEntry {
    /// Its desktop file id.
    pub id: DesktopId,
    /// Where its desktop file is (`%k`).
    pub path: PathBuf,
    /// Its name (`%c`); the id when the entry has none.
    pub name: String,
    /// Its icon (`%i`).
    pub icon: Option<String>,
    /// Its `Exec` value, with the key-file escapes taken.
    pub exec: Option<String>,
    /// Whether it wants a terminal.
    pub terminal: Terminal,
    /// How it is started.
    pub start: Start,
    /// The folder to run it in (`Path`).
    pub work_dir: Option<PathBuf>,
}

impl AppEntry {
    /// The app described by the desktop file `text`.
    pub fn from_text(id: DesktopId, path: &Path, text: &str) -> Result<AppEntry, EntryError> {
        let file =
            DesktopFile::parse(text, DESKTOP_ENTRY_CAP).map_err(|_| EntryError::Unreadable)?;
        AppEntry::from_file(id, path, &file)
    }

    /// The app of an already parsed desktop file.
    pub fn from_file(
        id: DesktopId,
        path: &Path,
        file: &DesktopFile,
    ) -> Result<AppEntry, EntryError> {
        let group = file.entry().ok_or(EntryError::NoEntry)?;
        let kind = group.string("Type").map(|kind| kind.into_owned());
        if kind.as_deref() != Some("Application") {
            return Err(EntryError::NotAnApp(id.to_string()));
        }
        let exec = group.string("Exec").map(|exec| exec.into_owned());
        let start = if group.is_true("DBusActivatable") {
            Start::Bus
        } else {
            Start::Command
        };
        if matches!(start, Start::Command) && exec.is_none() {
            return Err(EntryError::NoCommand(id.to_string()));
        }
        let text = |key: &str| {
            group
                .string(key)
                .map(|value| value.trim().to_owned())
                .filter(|value| !value.is_empty())
        };
        Ok(AppEntry {
            name: text("Name").unwrap_or_else(|| id.to_string()),
            icon: text("Icon"),
            terminal: if group.is_true("Terminal") {
                Terminal::Needed
            } else {
                Terminal::NotNeeded
            },
            work_dir: text("Path").map(PathBuf::from),
            id,
            path: path.to_owned(),
            exec,
            start,
        })
    }

    /// The app of an entry that `ds-helpers` found on disk; `None` when it is not an app.
    pub fn from_installed(entry: &ApplicationEntry) -> Option<AppEntry> {
        let id = DesktopId::new(entry.id.as_str()).ok()?;
        AppEntry::from_file(id, &entry.path, &entry.file).ok()
    }

    /// The bus name of a `DBusActivatable` app: its id without `.desktop`.
    pub fn bus_name(&self) -> &str {
        let id = self.id.as_str();
        id.strip_suffix(".desktop").unwrap_or(id)
    }
}

/// Every app under the `applications` folders `dirs`, most important first, each id once.
/// This reads the disk; the folders come from the caller.
pub fn load_apps(dirs: &[PathBuf]) -> Vec<AppEntry> {
    ds_helpers::load_entries(dirs)
        .iter()
        .filter_map(AppEntry::from_installed)
        .collect()
}

/// The check `ds_helpers::mime::Associations` takes for "is this app installed": it is one of `apps`.
pub fn installed_check(
    apps: &[AppEntry],
) -> impl Fn(&DesktopId) -> Installed + Send + Sync + 'static {
    let known: HashSet<DesktopId> = apps.iter().map(|app| app.id.clone()).collect();
    move |id| {
        if known.contains(id) {
            Installed::Yes
        } else {
            Installed::No
        }
    }
}
