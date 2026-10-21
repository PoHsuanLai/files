//! The edge: effects carried out against fakes and a scratch folder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};

use files_core::{
    Answer, ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Location, OpFailure, OpKind,
    OpPlan, Resolution, Transfer,
};
use files_fs::{
    FakeLister, FakePrefs, FakeWatcher, FolderEvent, ListError, Lister, NoPrefs, PrefsStore,
    Watcher,
};
use files_model::{Change, Effect, End, Input, ListFailure, OpEvent, OpenTarget};
use files_trash::{FakeTrash, Trash};
use files_ui::Opener;
use files_ui::edge::{Edge, Heard, Seams, change_of};
use tempfile::TempDir;
use tokio::sync::mpsc::UnboundedReceiver;

use super::support::{NoAppOpener, RecordingOpener, at, scratch_folder};

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
    watcher: Arc<FakeWatcher>,
    opener: Arc<RecordingOpener>,
    trash: Arc<FakeTrash>,
    pub(super) prefs: Arc<FakePrefs>,
    _bin: TempDir,
}

pub(super) fn rig() -> Rig {
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
fn the_trash_is_listed_as_rows_named_as_it_stores_them() {
    let mut rig = rig();
    let dir = scratch_folder();
    let folder = FolderPath::new(dir.path()).unwrap();
    let item = ItemRef::new(folder, name("notes.txt"));
    let thrown = rig.trash.trash(std::slice::from_ref(&item)).unwrap();

    rig.edge.run(vec![Effect::List(Location::Trash)]);
    let Heard::Input(Input::Listed(Location::Trash, Ok(rows))) = next(&mut rig.inbox) else {
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
        trash: Arc::new(FakeTrash::new("/nonexistent/never-used")),
        prefs: Arc::new(NoPrefs),
    };
    let (edge, mut inbox) = Edge::new(seams);
    let target = OpenTarget::new(folder("/p"), name("x.zzz"));
    edge.run(vec![Effect::OpenFiles(vec![target])]);
    assert_eq!(
        next(&mut inbox),
        Heard::Told("There is no app set to open \u{201c}x.zzz\u{201d}.".to_owned())
    );
}

/// Everything said until `found` is true of it; the thing itself.
fn until(inbox: &mut UnboundedReceiver<Heard>, found: impl Fn(&Heard) -> bool) -> Heard {
    loop {
        let heard = next(inbox);
        if found(&heard) {
            return heard;
        }
    }
}

fn finished(heard: &Heard) -> bool {
    matches!(heard, Heard::Input(Input::Op(OpEvent::Finished(_))))
}

/// How the change that just ended ended.
fn end_of(heard: Heard) -> End {
    let Heard::Input(Input::Op(OpEvent::Finished(outcome))) = heard else {
        panic!("not a finished change");
    };
    outcome.end
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
    assert_eq!(end_of(until(&mut rig.inbox, finished)), End::Done);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Changed(at(dir.path()), Change::Rescan))
    );
    assert!(dir.path().join("ideas.txt").exists());
    assert!(!dir.path().join("notes.txt").exists());
}

#[test]
fn renaming_onto_another_file_is_refused_and_the_failure_is_reported() {
    let mut rig = rig();
    let dir = scratch_folder();
    rig.edge.run(vec![Effect::Rename {
        folder: FolderPath::new(dir.path()).unwrap(),
        from: name("notes.txt"),
        to: name("report.pdf"),
    }]);
    assert_eq!(
        end_of(until(&mut rig.inbox, finished)),
        End::Failed(OpFailure::Taken)
    );
    assert_eq!(
        std::fs::read_to_string(dir.path().join("report.pdf")).unwrap(),
        "%PDF"
    );
    assert!(dir.path().join("notes.txt").exists());
}

#[test]
fn renaming_a_file_that_is_gone_is_reported() {
    let mut rig = rig();
    let dir = scratch_folder();
    rig.edge.run(vec![Effect::Rename {
        folder: FolderPath::new(dir.path()).unwrap(),
        from: name("nope.txt"),
        to: name("other.txt"),
    }]);
    assert_eq!(
        end_of(until(&mut rig.inbox, finished)),
        End::Failed(OpFailure::NotFound)
    );
}

#[test]
fn a_failure_the_model_tells_is_said_in_plain_words() {
    let mut rig = rig();
    rig.edge.run(vec![Effect::Tell(OpFailure::Taken)]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Told("That name is taken.".to_owned())
    );
}

#[test]
fn trashing_reports_each_step_rescans_what_changed_and_can_be_undone() {
    let mut rig = rig();
    let dir = scratch_folder();
    let here = FolderPath::new(dir.path()).unwrap();
    let item = ItemRef::new(here.clone(), name("notes.txt"));
    rig.edge.run(vec![Effect::Run(OpPlan::Trash {
        items: vec![item.clone()],
    })]);

    let started = until(&mut rig.inbox, |h| {
        matches!(h, Heard::Input(Input::Op(OpEvent::Started(_))))
    });
    let Heard::Input(Input::Op(OpEvent::Started(summary))) = started else {
        panic!("not started");
    };
    assert_eq!(summary.kind, OpKind::Trash);
    let done = until(&mut rig.inbox, finished);
    let Heard::Input(Input::Op(OpEvent::Finished(outcome))) = done else {
        panic!("not finished");
    };
    assert_eq!(outcome.end, End::Done);
    assert_eq!(outcome.undo.map(|s| s.kind), Some(OpKind::Trash));
    assert!(!dir.path().join("notes.txt").exists());
    let rescans = [next(&mut rig.inbox), next(&mut rig.inbox)];
    for place in [Location::Folder(here), Location::Trash] {
        let want = Heard::Input(Input::Changed(place, Change::Rescan));
        assert!(rescans.contains(&want), "{want:?} is not in {rescans:?}");
    }

    rig.edge.run(vec![Effect::Undo]);
    assert_eq!(end_of(until(&mut rig.inbox, finished)), End::Done);
    assert!(dir.path().join("notes.txt").exists());
}

#[test]
fn a_taken_name_is_asked_and_the_answer_goes_back_to_the_work() {
    let mut rig = rig();
    let dir = scratch_folder();
    let from = FolderPath::new(dir.path().join("Projects")).unwrap();
    let to = FolderPath::new(dir.path()).unwrap();
    std::fs::write(dir.path().join("Projects").join("notes.txt"), "other").unwrap();
    let plan = OpPlan::Copy(Transfer::new(
        vec![ItemRef::new(from, name("notes.txt"))],
        to,
    ));
    rig.edge.run(vec![Effect::Run(plan)]);

    let asked = until(&mut rig.inbox, |h| {
        matches!(h, Heard::Input(Input::Op(OpEvent::Asking(_))))
    });
    let Heard::Input(Input::Op(OpEvent::Asking(conflict))) = asked else {
        panic!("not asked");
    };
    assert_eq!(conflict.item, name("notes.txt"));
    rig.edge
        .run(vec![Effect::Answer(Resolution::once(Answer::KeepBoth))]);
    assert_eq!(end_of(until(&mut rig.inbox, finished)), End::Done);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("notes 2.txt")).unwrap(),
        "other"
    );
}

#[test]
fn a_move_to_the_trash_is_announced_so_the_window_can_offer_to_undo_it() {
    let mut rig = rig();
    let summary = files_core::Summary::new(OpKind::Trash, 1, Some(name("x")));
    rig.edge.run(vec![Effect::Announce(summary.clone())]);
    assert_eq!(next(&mut rig.inbox), Heard::Announced(summary));
}
