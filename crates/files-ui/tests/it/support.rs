//! Fakes and a scratch folder for the tests: nothing here touches the real home or session.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use ds::prelude::SystemPrefs;
use ds_harness::{Harness, HarnessConfig, Viewport};
use files_core::{FolderPath, Location};
use files_fs::{DiskLister, FakeWatcher};
use files_ui::edge::{DiskRenamer, Seams};
use files_ui::{AppearanceSource, Launch, Opener, Zone};
use tempfile::TempDir;

/// An opener that remembers what it was asked to open.
#[derive(Debug, Default)]
pub struct RecordingOpener {
    opened: Mutex<Vec<PathBuf>>,
}

impl RecordingOpener {
    /// Everything opened so far.
    pub fn opened(&self) -> Vec<PathBuf> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Opener for RecordingOpener {
    fn open(&self, targets: Vec<PathBuf>) {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .extend(targets);
    }
}

/// A scratch folder holding `notes.txt`, `Projects/inner.txt`, `report.pdf`, `zebra.png` and a
/// hidden `.secret`. By name they sort `notes.txt`, `Projects`, `report.pdf`, `zebra.png`.
pub fn scratch_folder() -> TempDir {
    let dir = tempfile::tempdir().unwrap();
    std::fs::write(dir.path().join("notes.txt"), "hello").unwrap();
    std::fs::create_dir(dir.path().join("Projects")).unwrap();
    std::fs::write(dir.path().join("Projects").join("inner.txt"), "x").unwrap();
    std::fs::write(dir.path().join("report.pdf"), "%PDF").unwrap();
    std::fs::write(dir.path().join("zebra.png"), "png").unwrap();
    std::fs::write(dir.path().join(".secret"), "s").unwrap();
    dir
}

/// `path` as a location.
pub fn at(path: &Path) -> Location {
    Location::Folder(FolderPath::new(path).unwrap())
}

/// A window on `start` over the real disk of the scratch folder, a watcher that watches nothing,
/// UTC dates and a look read from nowhere.
pub fn launch(start: &Path, config: &Path, opener: &Arc<RecordingOpener>) -> Launch {
    let seams = Seams {
        lister: Arc::new(DiskLister),
        watcher: Arc::new(FakeWatcher::new()),
        opener: Arc::clone(opener) as Arc<dyn Opener>,
        renamer: Arc::new(DiskRenamer),
    };
    Launch::new(at(start), Arc::clone(opener) as Arc<dyn Opener>)
        .with_seams(seams)
        .with_appearance(AppearanceSource::scratch(
            config.to_path_buf(),
            SystemPrefs::default(),
        ))
        .with_zone(Zone::Utc)
}

/// The window in a harness, laid out as mapped.
pub fn harness(launch: Launch) -> Harness {
    let view = Viewport {
        width: 1000,
        height: 600,
        scale_percent: 100,
    };
    let mut harness = Harness::new(
        files_ui::App,
        HarnessConfig::new(view).with_context(launch),
    );
    harness.map();
    harness
}
