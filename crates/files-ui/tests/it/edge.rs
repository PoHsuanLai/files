//! The edge: effects carried out against fakes and a scratch folder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, FolderPrefs, Location};
use files_fs::{FakeLister, FakeWatcher, FolderEvent, ListError, Lister, Watcher};
use files_model::{Change, Effect, Input, ListFailure, OpenTarget};
use files_ui::edge::{DiskRenamer, Edge, Heard, Seams, change_of};
use files_ui::{Opener, Renamer};
use tokio::sync::mpsc::UnboundedReceiver;

use super::support::{NoAppOpener, RecordingOpener, at, scratch_folder};

fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

fn file(text: &str) -> Entry {
    Entry::new(name(text), EntryKind::File(ByteSize(3)))
}

struct Rig {
    edge: Edge,
    inbox: UnboundedReceiver<Heard>,
    lister: Arc<FakeLister>,
    watcher: Arc<FakeWatcher>,
    opener: Arc<RecordingOpener>,
}

fn rig() -> Rig {
    let lister = Arc::new(FakeLister::new());
    let watcher = Arc::new(FakeWatcher::new());
    let opener = Arc::new(RecordingOpener::default());
    let seams = Seams {
        lister: Arc::clone(&lister) as Arc<dyn Lister>,
        watcher: Arc::clone(&watcher) as Arc<dyn Watcher + Send + Sync>,
        opener: Arc::clone(&opener) as Arc<dyn Opener>,
        renamer: Arc::new(DiskRenamer) as Arc<dyn Renamer>,
    };
    let (edge, inbox) = Edge::new(seams);
    Rig {
        edge,
        inbox,
        lister,
        watcher,
        opener,
    }
}

/// The next thing the edge said, waiting up to five seconds for a worker.
fn next(inbox: &mut UnboundedReceiver<Heard>) -> Heard {
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

fn folder(path: &str) -> FolderPath {
    FolderPath::new(path).unwrap()
}

#[test]
fn listing_a_folder_answers_with_its_entries() {
    let mut rig = rig();
    let here = folder("/p");
    rig.lister.set(here.clone(), Ok(vec![file("a"), file("b")]));
    let place = Location::Folder(here);
    rig.edge.run(vec![Effect::List(place.clone())]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Listed(place, Ok(vec![file("a"), file("b")])))
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
        rig.edge.run(vec![Effect::List(place.clone())]);
        assert_eq!(
            next(&mut rig.inbox),
            Heard::Input(Input::Listed(place, Err(want))),
            "{why}"
        );
    }
}

#[test]
fn the_trash_is_not_listed_yet() {
    let mut rig = rig();
    rig.edge.run(vec![Effect::List(Location::Trash)]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Listed(Location::Trash, Err(ListFailure::Other)))
    );
}

#[test]
fn folders_are_remembered_by_default_for_now() {
    let mut rig = rig();
    let place = Location::Folder(folder("/p"));
    rig.edge.run(vec![
        Effect::LoadPrefs(place.clone()),
        Effect::SavePrefs(place.clone(), FolderPrefs::default()),
    ]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::PrefsLoaded(place, FolderPrefs::default()))
    );
    assert!(rig.inbox.try_recv().is_err(), "saving says nothing");
}

#[test]
fn watching_follows_a_folder_until_it_is_left() {
    let mut rig = rig();
    let here = folder("/p");
    let place = Location::Folder(here.clone());
    rig.lister.set(here.clone(), Ok(vec![file("a")]));

    rig.edge.run(vec![Effect::Watch(place.clone())]);
    assert_eq!(rig.watcher.watched(), vec![here.clone()]);

    rig.watcher.push(&here, FolderEvent::Changed(name("a")));
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Changed(place.clone(), Change::Updated(file("a"))))
    );

    rig.edge.run(vec![Effect::Unwatch(place)]);
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
    rig.edge.run(vec![Effect::OpenFiles(vec![target])]);
    assert_eq!(rig.opener.opened(), vec![PathBuf::from("/p/a.txt")]);
}

#[test]
fn a_file_no_app_opens_is_told_in_words() {
    let seams = Seams {
        lister: Arc::new(FakeLister::new()),
        watcher: Arc::new(FakeWatcher::new()),
        opener: Arc::new(NoAppOpener),
        renamer: Arc::new(DiskRenamer),
    };
    let (edge, mut inbox) = Edge::new(seams);
    let target = OpenTarget::new(folder("/p"), name("x.zzz"));
    edge.run(vec![Effect::OpenFiles(vec![target])]);
    assert_eq!(
        next(&mut inbox),
        Heard::Told("There is no app set to open \u{201c}x.zzz\u{201d}.".to_owned())
    );
}

#[test]
fn renaming_moves_the_file_and_asks_for_the_listing_again() {
    let mut rig = rig();
    let dir = scratch_folder();
    let here = FolderPath::new(dir.path()).unwrap();
    rig.edge.run(vec![Effect::Rename {
        folder: here,
        from: name("notes.txt"),
        to: name("ideas.txt"),
    }]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Changed(at(dir.path()), Change::Rescan))
    );
    assert!(dir.path().join("ideas.txt").exists());
    assert!(!dir.path().join("notes.txt").exists());
}

#[test]
fn renaming_onto_another_file_is_refused_in_plain_words() {
    let mut rig = rig();
    let dir = scratch_folder();
    rig.edge.run(vec![Effect::Rename {
        folder: FolderPath::new(dir.path()).unwrap(),
        from: name("notes.txt"),
        to: name("report.pdf"),
    }]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Told("That name is taken.".to_owned())
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("report.pdf")).unwrap(),
        "%PDF"
    );
    assert!(dir.path().join("notes.txt").exists());
}

#[test]
fn renaming_a_file_that_is_gone_says_so() {
    let mut rig = rig();
    let dir = scratch_folder();
    rig.edge.run(vec![Effect::Rename {
        folder: FolderPath::new(dir.path()).unwrap(),
        from: name("nope.txt"),
        to: name("other.txt"),
    }]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Told("That item can't be found any more.".to_owned())
    );
}
