//! The window's seams to the outside.

use std::fmt;
use std::sync::Arc;

use anyview_platform::ThumbnailCache;
use files_fs::{Clouds, DiskLister, Lister, NoClouds, NoPrefs, NotifyWatcher, PrefsStore, Watcher};
use files_trash::{SystemTrash, Trash};

use super::{Apps, NoApps, NoThumbnails, NoVolumes, Opener, Volumes};

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
    /// Which apps open what, for Open With.
    pub apps: Arc<dyn Apps>,
    /// The computer's disks: listing, opening and ejecting them.
    pub volumes: Arc<dyn Volumes>,
    /// The cloud drives: listing them, where their files stand, downloading and keeping.
    pub clouds: Arc<dyn Clouds>,
    /// The thumbnails the desktop shares, which the icon view reads and adds to.
    pub thumbnails: Arc<dyn ThumbnailCache + Send + Sync>,
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
            apps: Arc::new(NoApps),
            volumes: Arc::new(NoVolumes),
            clouds: Arc::new(NoClouds),
            thumbnails: Arc::new(NoThumbnails),
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

    /// The same seams over the cloud drives `clouds` knows.
    pub fn with_clouds(self, clouds: Arc<dyn Clouds>) -> Seams {
        Seams { clouds, ..self }
    }

    /// The same seams sharing thumbnails through `thumbnails`.
    pub fn with_thumbnails(self, thumbnails: Arc<dyn ThumbnailCache + Send + Sync>) -> Seams {
        Seams { thumbnails, ..self }
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
