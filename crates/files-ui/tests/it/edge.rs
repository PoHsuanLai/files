//! The edge: effects carried out against fakes and a scratch folder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Location};
use files_fs::{
    FakeLister, FakePrefs, FakeWatcher, FolderEvent, ListError, Lister, NoPrefs, PrefsStore,
    Watcher,
};
use files_model::{Change, Effect, Input, ListFailure, OpenTarget, TabId};
use files_trash::{FakeTrash, Trash};
use files_ui::edge::{Edge, Heard, Seams, change_of};
use files_ui::{NoApps, NoThumbnails, NoVolumes, Opener, Volumes};
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

use super::support::{NoAppOpener, RecordingOpener, scratch_folder};

pub(super) fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

pub(super) fn file(text: &str) -> Entry {
    Entry::new(name(text), EntryKind::File(ByteSize(3)))
}

pub(super) struct Rig {
    pub(super) edge: Edge,
    pub(super) inbox: UnboundedReceiver<Heard>,
    pub(super) lister: Arc<FakeLister>,
    pub(super) watcher: Arc<FakeWatcher>,
    pub(super) opener: Arc<RecordingOpener>,
    trash: Arc<FakeTrash>,
    pub(super) prefs: Arc<FakePrefs>,
    _bin: TempDir,
}

pub(super) fn rig() -> Rig {
    rig_over(Arc::new(NoVolumes))
}

/// A rig whose computer has the disks `volumes` holds.
pub(super) fn rig_over(volumes: Arc<dyn Volumes>) -> Rig {
    let lister = Arc::new(FakeLister::new());
    let watcher = Arc::new(FakeWatcher::new());
    let opener = Arc::new(RecordingOpener::default());
    let bin = tempfile::tempdir().unwrap();
    let trash = Arc::new(FakeTrash::new(bin.path().join("bin")));
    let prefs = Arc::new(FakePrefs::new());
    let seams = Seams {
        lister: Arc::clone(&lister) as Arc<dyn Lister>,
        watcher: Arc::clone(&watcher) as Arc<dyn Watcher + Send + Sync>,
        opener: Arc::clone(&opener) as Arc<dyn Opener>,
        trash: Arc::clone(&trash) as Arc<dyn Trash>,
        prefs: Arc::clone(&prefs) as Arc<dyn PrefsStore>,
        apps: Arc::new(NoApps),
        volumes,
        thumbnails: Arc::new(NoThumbnails),
    };
    let (edge, inbox) = Edge::new(seams);
    Rig {
        edge,
        inbox,
        lister,
        watcher,
        opener,
        trash,
        prefs,
        _bin: bin,
    }
}

/// The next thing the edge said, waiting up to five seconds for a worker.
pub(super) fn next(inbox: &mut UnboundedReceiver<Heard>) -> Heard {
    let started = Instant::now();
    loop {
        if let Ok(heard) = inbox.try_recv() {
            return heard;
        }
        assert!(
            started.elapsed() < Duration::from_secs(5),
            "nothing was said"
        );
        std::thread::sleep(Duration::from_millis(5));
    }
}

pub(super) fn folder(path: &str) -> FolderPath {
    FolderPath::new(path).unwrap()
}

#[test]
fn listing_a_folder_answers_with_its_entries() {
    let mut rig = rig();
    let here = folder("/p");
    rig.lister.set(here.clone(), Ok(vec![file("a"), file("b")]));
    let place = Location::Folder(here);
    rig.edge
        .run_in(TabId::FIRST, vec![Effect::List(place.clone())]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::Listed(place, Ok(vec![file("a"), file("b")]))
        )
    );
}

#[test]
fn a_listing_that_fails_says_why_in_the_models_terms() {
    let cases = [
        ("gone", Err(ListError::NotFound), ListFailure::NotFound),
        (
            "not allowed",
            Err(ListError::NoPermission),
            ListFailure::NoPermission,
        ),
        (
            "a file",
            Err(ListError::NotAFolder),
            ListFailure::NotAFolder,
        ),
        (
            "anything else",
            Err(ListError::Other(std::io::ErrorKind::TimedOut)),
            ListFailure::Other,
        ),
    ];
    for (why, result, want) in cases {
        let mut rig = rig();
        let here = folder("/p");
        rig.lister.set(here.clone(), result);
        let place = Location::Folder(here);
        rig.edge
            .run_in(TabId::FIRST, vec![Effect::List(place.clone())]);
        assert_eq!(
            next(&mut rig.inbox),
            Heard::Tab(TabId::FIRST, Input::Listed(place, Err(want))),
            "{why}"
        );
    }
}

#[test]
fn the_trash_is_listed_as_rows_named_as_it_stores_them() {
    let mut rig = rig();
    let dir = scratch_folder();
    let folder = FolderPath::new(dir.path()).unwrap();
    let item = ItemRef::new(folder, name("notes.txt"));
    let thrown = rig.trash.trash(std::slice::from_ref(&item)).unwrap();

    rig.edge
        .run_in(TabId::FIRST, vec![Effect::List(Location::Trash)]);
    let Heard::Tab(TabId::FIRST, Input::Listed(Location::Trash, Ok(rows))) = next(&mut rig.inbox)
    else {
        panic!("the Trash was not listed");
    };
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].name, name("notes.txt"));
    assert_eq!(rows[0].trashed, Some(thrown[0].clone()));
    assert_eq!(rows[0].shown_name(), &name("notes.txt"));
}

#[test]
fn watching_follows_a_folder_until_it_is_left() {
    let mut rig = rig();
    let here = folder("/p");
    let place = Location::Folder(here.clone());
    rig.lister.set(here.clone(), Ok(vec![file("a")]));

    rig.edge
        .run_in(TabId::FIRST, vec![Effect::Watch(place.clone())]);
    assert_eq!(rig.watcher.watched(), vec![here.clone()]);

    rig.watcher.push(&here, FolderEvent::Changed(name("a")));
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::Changed(place.clone(), Change::Updated(file("a")))
        )
    );

    rig.edge.run_in(TabId::FIRST, vec![Effect::Unwatch(place)]);
    assert!(rig.watcher.watched().is_empty());
}

#[test]
fn a_watch_event_becomes_the_change_it_stands_for() {
    let lister = FakeLister::new();
    let here = folder("/p");
    lister.set(here.clone(), Ok(vec![file("kept")]));
    let cases = [
        (
            "added",
            FolderEvent::Added(name("kept")),
            Change::Added(file("kept")),
        ),
        (
            "changed",
            FolderEvent::Changed(name("kept")),
            Change::Updated(file("kept")),
        ),
        (
            "added and gone again",
            FolderEvent::Added(name("late")),
            Change::Removed(name("late")),
        ),
        (
            "changed and gone",
            FolderEvent::Changed(name("late")),
            Change::Removed(name("late")),
        ),
        (
            "removed",
            FolderEvent::Removed(name("kept")),
            Change::Removed(name("kept")),
        ),
        ("missed some", FolderEvent::Rescan, Change::Rescan),
        ("the folder went", FolderEvent::Gone, Change::Rescan),
    ];
    for (why, event, want) in cases {
        assert_eq!(change_of(&lister, &here, event), want, "{why}");
    }
}

#[test]
fn an_entry_that_cannot_be_read_again_asks_for_a_rescan() {
    let lister = FakeLister::new();
    let here = folder("/p");
    lister.set(here.clone(), Err(ListError::NoPermission));
    assert_eq!(
        change_of(&lister, &here, FolderEvent::Added(name("x"))),
        Change::Rescan
    );
}

#[test]
fn files_are_handed_to_the_opener_by_path() {
    let rig = rig();
    let target = OpenTarget::new(folder("/p"), name("a.txt"));
    rig.edge
        .run_in(TabId::FIRST, vec![Effect::OpenFiles(vec![target])]);
    assert_eq!(rig.opener.opened(), vec![PathBuf::from("/p/a.txt")]);
}

#[test]
fn a_file_no_app_opens_is_told_in_words() {
    let seams = Seams {
        lister: Arc::new(FakeLister::new()),
        watcher: Arc::new(FakeWatcher::new()),
        opener: Arc::new(NoAppOpener),
        trash: Arc::new(FakeTrash::new("/nonexistent/never-used")),
        prefs: Arc::new(NoPrefs),
        apps: Arc::new(NoApps),
        volumes: Arc::new(NoVolumes),
        thumbnails: Arc::new(NoThumbnails),
    };
    let (edge, mut inbox) = Edge::new(seams);
    let target = OpenTarget::new(folder("/p"), name("x.zzz"));
    edge.run_in(TabId::FIRST, vec![Effect::OpenFiles(vec![target])]);
    assert_eq!(
        next(&mut inbox),
        Heard::Told("There is no app set to open \u{201c}x.zzz\u{201d}.".to_owned())
    );
}
