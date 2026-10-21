//! Fakes and a scratch folder for the tests: nothing here touches the real home or session.

use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

use ds::prelude::SystemPrefs;
use ds_harness::{Harness, HarnessConfig, Viewport};
use files_core::{FolderPath, Location};
use files_fs::{DiskLister, FakePrefs, FakeTagger, FakeWatcher};
use files_model::AppPick;
use files_trash::FakeTrash;
use files_ui::edge::Seams;
use files_ui::{
    AppChoiceView, AppRole, AppearanceSource, Apps, Launch, NoVolumes, OpenAs, OpenFailure, Opener,
    Zone,
};
use tempfile::TempDir;

/// An opener that remembers what it was asked to open, and how.
#[derive(Debug, Default)]
pub struct RecordingOpener {
    opened: Mutex<Vec<(Vec<PathBuf>, OpenAs)>>,
}

impl RecordingOpener {
    /// Every path opened so far, in order, whichever app it went to.
    pub fn opened(&self) -> Vec<PathBuf> {
        self.calls()
            .into_iter()
            .flat_map(|(paths, _)| paths)
            .collect()
    }

    /// Every call so far: the paths and how they were to be opened.
    pub fn calls(&self) -> Vec<(Vec<PathBuf>, OpenAs)> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Opener for RecordingOpener {
    fn open(&self, targets: Vec<PathBuf>, with: OpenAs) -> Result<(), OpenFailure> {
        self.opened
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push((targets, with));
        Ok(())
    }
}

/// Apps that answer from a list. For any file `Viewer` is the default and `editor` and `Zed` open
/// it too; every installed app is `Zed`, `Calculator` and `Viewer`.
#[derive(Debug, Clone, Copy, Default)]
pub struct FakeApps;

fn app(id: &str, name: &str, role: AppRole) -> AppChoiceView {
    AppChoiceView {
        pick: AppPick::new(id),
        name: name.to_owned(),
        role,
    }
}

impl Apps for FakeApps {
    fn apps_for(&self, _targets: &[PathBuf]) -> Vec<AppChoiceView> {
        vec![
            app("zed.desktop", "Zed", AppRole::Other),
            app("viewer.desktop", "Viewer", AppRole::Default),
            app("editor.desktop", "editor", AppRole::Other),
        ]
    }

    fn all_apps(&self) -> Vec<AppChoiceView> {
        vec![
            app("zed.desktop", "Zed", AppRole::Other),
            app("calc.desktop", "Calculator", AppRole::Other),
            app("viewer.desktop", "Viewer", AppRole::Default),
        ]
    }
}

/// An opener that finds no app for anything.
#[derive(Debug, Clone, Copy)]
pub struct NoAppOpener;

impl Opener for NoAppOpener {
    fn open(&self, targets: Vec<PathBuf>, _with: OpenAs) -> Result<(), OpenFailure> {
        let first = targets.first().and_then(|path| path.file_name());
        let shown = first.map(|name| name.to_string_lossy().into_owned());
        Err(OpenFailure(format!(
            "There is no app set to open \u{201c}{}\u{201d}.",
            shown.unwrap_or_default()
        )))
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

/// A window on `start` over the real disk of the scratch folder, a Trash kept in `config`'s
/// folder (the same disk, so a move to it is a rename), a watcher that watches nothing, UTC dates
/// and a look read from nowhere.
pub fn launch(start: &Path, config: &Path, opener: &Arc<RecordingOpener>) -> Launch {
    let trash = Arc::new(FakeTrash::new(config.join("trash")));
    launch_with(at(start), config, opener, trash)
}

/// The same, starting at `start` over the given `trash`, which the test keeps hold of.
pub fn launch_with(
    start: Location,
    config: &Path,
    opener: &Arc<RecordingOpener>,
    trash: Arc<FakeTrash>,
) -> Launch {
    launch_remembering(start, config, opener, trash, Arc::new(FakePrefs::new()))
}

/// The same, over the given `prefs`, which stand for the state folder a relaunch finds again.
pub fn launch_remembering(
    start: Location,
    config: &Path,
    opener: &Arc<RecordingOpener>,
    trash: Arc<FakeTrash>,
    prefs: Arc<FakePrefs>,
) -> Launch {
    launch_tagging(
        start,
        config,
        opener,
        trash,
        prefs,
        Arc::new(FakeTagger::new()),
    )
}

/// The same, keeping tags in the given `tagger`, which the test keeps hold of. The tags it holds
/// show on the rows the real disk lists.
pub fn launch_tagging(
    start: Location,
    config: &Path,
    opener: &Arc<RecordingOpener>,
    trash: Arc<FakeTrash>,
    prefs: Arc<FakePrefs>,
    tagger: Arc<FakeTagger>,
) -> Launch {
    let seams = Seams {
        lister: Arc::new(DiskLister),
        watcher: Arc::new(FakeWatcher::new()),
        opener: Arc::clone(opener) as Arc<dyn Opener>,
        trash,
        prefs,
        tagger,
        apps: Arc::new(FakeApps),
        volumes: Arc::new(NoVolumes),
    };
    Launch::new(start, Arc::clone(opener) as Arc<dyn Opener>)
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
    let mut harness = Harness::new(files_ui::App, HarnessConfig::new(view).with_context(launch));
    harness.map();
    harness
}
