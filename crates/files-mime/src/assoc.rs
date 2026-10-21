//! Which apps open a type, and which one is the default.

use std::fmt;

use files_core::MimeType;

use crate::db::MimeDb;
use crate::desktop_id::DesktopId;
use crate::mimeapps::{MimeApps, MimeCache};

/// Whether an app is installed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Installed {
    /// Its desktop file exists.
    Yes,
    /// It does not.
    No,
}

type InstalledCheck = dyn Fn(&DesktopId) -> Installed + Send + Sync;

/// The `mimeapps.list` files and `mimeinfo.cache` files, in precedence order, and the check for
/// an app being installed.
pub struct Associations {
    files: Vec<MimeApps>,
    caches: Vec<MimeCache>,
    installed: Box<InstalledCheck>,
}

impl fmt::Debug for Associations {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Associations")
            .field("files", &self.files.len())
            .field("caches", &self.caches.len())
            .finish_non_exhaustive()
    }
}

impl Associations {
    /// `files` and `caches` in precedence order, highest first.
    pub fn new(
        files: Vec<MimeApps>,
        caches: Vec<MimeCache>,
        installed: impl Fn(&DesktopId) -> Installed + Send + Sync + 'static,
    ) -> Associations {
        Associations { files, caches, installed: Box::new(installed) }
    }

    /// Whether `id` is removed for `mime` by any of the first `levels` files.
    fn removed_within(&self, levels: usize, mime: &MimeType, id: &DesktopId) -> bool {
        self.files.iter().take(levels).any(|file| file.removed_for(mime).contains(id))
    }

    /// The installed apps for exactly `mime`: defaults (not removed in their own file), then each
    /// file's added apps (not removed in that file or above), then the caches' (not removed at all).
    fn direct_apps(&self, mime: &MimeType) -> Vec<DesktopId> {
        let mut found: Vec<DesktopId> = Vec::new();
        for file in &self.files {
            let removed = file.removed_for(mime);
            found.extend(file.defaults_for(mime).iter().filter(|id| !removed.contains(id)).cloned());
        }
        for (level, file) in self.files.iter().enumerate() {
            found.extend(
                file.added_for(mime)
                    .iter()
                    .filter(|id| !self.removed_within(level + 1, mime, id))
                    .cloned(),
            );
        }
        for cache in &self.caches {
            found.extend(
                cache
                    .apps_for(mime)
                    .iter()
                    .filter(|id| !self.removed_within(self.files.len(), mime, id))
                    .cloned(),
            );
        }
        let mut apps: Vec<DesktopId> = Vec::new();
        for id in found {
            if !apps.contains(&id) && (self.installed)(&id) == Installed::Yes {
                apps.push(id);
            }
        }
        apps
    }

    /// `mime` and then the types it is a special case of, nearest first.
    fn chain(mime: &MimeType, db: &MimeDb) -> Vec<MimeType> {
        let mut chain = vec![db.canonical(mime)];
        chain.extend(db.ancestors(mime));
        chain
    }

    /// The app that opens `mime` by default: the first installed default in precedence order,
    /// else the first installed added or cached app; the same again for each parent type.
    pub fn default_for(&self, mime: &MimeType, db: &MimeDb) -> Option<DesktopId> {
        Self::chain(mime, db).iter().find_map(|kind| self.direct_apps(kind).into_iter().next())
    }

    /// The apps that open `mime`, default first, with no repeats; the parent types' apps follow.
    pub fn apps_for(&self, mime: &MimeType, db: &MimeDb) -> Vec<DesktopId> {
        let mut apps: Vec<DesktopId> = Vec::new();
        for kind in Self::chain(mime, db) {
            for id in self.direct_apps(&kind) {
                if !apps.contains(&id) {
                    apps.push(id);
                }
            }
        }
        apps
    }

    /// The default app for folders: the file manager.
    pub fn file_manager_default(&self) -> Option<DesktopId> {
        let folder = MimeType::new(MimeType::FOLDER).ok()?;
        self.direct_apps(&folder).into_iter().next()
    }
}
