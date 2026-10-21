//! The window's seams to the outside.

use std::fmt;
use std::sync::Arc;

use files_fs::{
    DiskLister, Lister, NoPrefs, NotifyWatcher, PrefsStore, Tagger, Watcher, XattrTagger,
};
use files_trash::{SystemTrash, Trash};

use super::{Apps, NoApps, NoVolumes, Opener, Volumes};

/// The window's seams to the outside, each behind a trait so a test gives its own.
#[derive(Clone)]
pub struct Seams {
    /// Reads folders.
    pub lister: Arc<dyn Lister>,
    /// Follows folders.
    pub watcher: Arc<dyn Watcher + Send + Sync>,
    /// Hands files to the apps that open them.
    pub opener: Arc<dyn Opener>,
    /// The Trash: moving to it, listing it, putting back, erasing.
    pub trash: Arc<dyn Trash>,
    /// Where each folder's view and sort are remembered.
    pub prefs: Arc<dyn PrefsStore>,
    /// The tags of files: read with every listing, written by the tag menu.
    pub tagger: Arc<dyn Tagger>,
    /// Which apps open what, for Open With.
    pub apps: Arc<dyn Apps>,
    /// The computer's disks: listing, opening and ejecting them.
    pub volumes: Arc<dyn Volumes>,
}

impl Seams {
    /// The real disk and Trash, with `opener` to open files.
    pub fn disk(opener: Arc<dyn Opener>) -> Seams {
        Seams {
            lister: Arc::new(DiskLister),
            watcher: Arc::new(NotifyWatcher),
            opener,
            trash: Arc::new(SystemTrash),
            prefs: Arc::new(NoPrefs),
            tagger: Arc::new(XattrTagger),
            apps: Arc::new(NoApps),
            volumes: Arc::new(NoVolumes),
        }
    }

    /// The same seams offering the apps `apps` knows in Open With.
    pub fn with_apps(self, apps: Arc<dyn Apps>) -> Seams {
        Seams { apps, ..self }
    }

    /// The same seams over the disks `volumes` knows.
    pub fn with_volumes(self, volumes: Arc<dyn Volumes>) -> Seams {
        Seams { volumes, ..self }
    }

    /// The same seams keeping tags with `tagger`.
    pub fn with_tagger(self, tagger: Arc<dyn Tagger>) -> Seams {
        Seams { tagger, ..self }
    }

    /// The same seams remembering folder views in `prefs`.
    pub fn with_prefs(self, prefs: Arc<dyn PrefsStore>) -> Seams {
        Seams { prefs, ..self }
    }
}

impl fmt::Debug for Seams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Seams")
    }
}
