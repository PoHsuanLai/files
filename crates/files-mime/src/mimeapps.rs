//! `mimeapps.list` and `mimeinfo.cache`: which apps go with which type.

use std::collections::HashMap;

use files_core::MimeType;

use crate::desktop_id::DesktopId;
use crate::text::key_lines;

type Table = HashMap<MimeType, Vec<DesktopId>>;

fn desktop_ids(value: &str) -> Vec<DesktopId> {
    value.split(';').filter_map(|id| DesktopId::new(id.trim()).ok()).collect()
}

fn lookup<'a>(table: &'a Table, mime: &MimeType) -> &'a [DesktopId] {
    table.get(mime).map(Vec::as_slice).unwrap_or_default()
}

/// One `mimeapps.list` file.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MimeApps {
    default: Table,
    added: Table,
    removed: Table,
}

impl MimeApps {
    /// The three groups of one file's text; other groups and bad lines are skipped.
    pub fn parse(text: &str) -> MimeApps {
        let mut apps = MimeApps::default();
        for line in key_lines(text) {
            let table = match line.group {
                "Default Applications" => &mut apps.default,
                "Added Associations" => &mut apps.added,
                "Removed Associations" => &mut apps.removed,
                _ => continue,
            };
            let Ok(mime) = MimeType::new(line.key) else { continue };
            table.entry(mime).or_default().extend(desktop_ids(line.value));
        }
        apps
    }

    /// The apps listed under `[Default Applications]` for `mime`, best first.
    pub fn defaults_for(&self, mime: &MimeType) -> &[DesktopId] {
        lookup(&self.default, mime)
    }

    /// The apps listed under `[Added Associations]` for `mime`.
    pub fn added_for(&self, mime: &MimeType) -> &[DesktopId] {
        lookup(&self.added, mime)
    }

    /// The apps listed under `[Removed Associations]` for `mime`.
    pub fn removed_for(&self, mime: &MimeType) -> &[DesktopId] {
        lookup(&self.removed, mime)
    }
}

/// One `mimeinfo.cache` file: what the installed apps say they open.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct MimeCache {
    apps: Table,
}

impl MimeCache {
    /// The `[MIME Cache]` group of one file's text.
    pub fn parse(text: &str) -> MimeCache {
        let mut apps = Table::new();
        for line in key_lines(text).into_iter().filter(|line| line.group == "MIME Cache") {
            let Ok(mime) = MimeType::new(line.key) else { continue };
            apps.entry(mime).or_default().extend(desktop_ids(line.value));
        }
        MimeCache { apps }
    }

    /// The apps that say they open `mime`, in file order.
    pub fn apps_for(&self, mime: &MimeType) -> &[DesktopId] {
        lookup(&self.apps, mime)
    }
}
