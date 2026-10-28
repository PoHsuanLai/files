//! The edge's queue of changes: rename, trash, undo and a taken name, reported and rescanned.

use files_core::{
    Answer, Finish, FolderPath, ItemRef, Location, OpEvent, OpFailure, OpKind, OpPlan, Resolution,
    Transfer,
};
use files_model::{Change, Effect, Input, TabId};
use files_ui::edge::Heard;
use tokio::sync::mpsc::UnboundedReceiver;

use super::edge::{name, next, rig};
use super::support::{at, scratch_folder};

/// Everything said until `found` is true of it; the thing itself.
pub(super) fn until(inbox: &mut UnboundedReceiver<Heard>, found: impl Fn(&Heard) -> bool) -> Heard {
    loop {
        let heard = next(inbox);
        if found(&heard) {
            return heard;
        }
    }
}

pub(super) fn finished(heard: &Heard) -> bool {
    matches!(
        heard,
        Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Finished(_)))
    )
}

/// How the change that just ended ended.
pub(super) fn finish_of(heard: Heard) -> Finish {
    let Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Finished(outcome))) = heard else {
        panic!("not a finished change");
    };
    outcome.finish
}

#[test]
fn renaming_moves_the_file_and_asks_for_the_listing_again() {
    let mut rig = rig();
    let dir = scratch_folder();
    let here = FolderPath::new(dir.path()).unwrap();
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Rename {
            folder: here,
            from: name("notes.txt"),
            to: name("ideas.txt"),
        }],
    );
    assert_eq!(finish_of(until(&mut rig.inbox, finished)), Finish::Done);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::All(Input::Changed(at(dir.path()), Change::Rescan))
    );
    assert!(dir.path().join("ideas.txt").exists());
    assert!(!dir.path().join("notes.txt").exists());
}

#[test]
fn renaming_onto_another_file_is_refused_and_the_failure_is_reported() {
    let mut rig = rig();
    let dir = scratch_folder();
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Rename {
            folder: FolderPath::new(dir.path()).unwrap(),
            from: name("notes.txt"),
            to: name("report.pdf"),
        }],
    );
    assert_eq!(
        finish_of(until(&mut rig.inbox, finished)),
        Finish::Failed(OpFailure::Taken)
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
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Rename {
            folder: FolderPath::new(dir.path()).unwrap(),
            from: name("nope.txt"),
            to: name("other.txt"),
        }],
    );
    assert_eq!(
        finish_of(until(&mut rig.inbox, finished)),
        Finish::Failed(OpFailure::NotFound)
    );
}

#[test]
fn a_failure_the_model_tells_is_said_in_plain_words() {
    let mut rig = rig();
    rig.edge
        .run_in(TabId::FIRST, vec![Effect::Tell(OpFailure::Taken)]);
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
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Run(OpPlan::Trash {
            items: vec![item.clone()],
        })],
    );

    let started = until(&mut rig.inbox, |h| {
        matches!(h, Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Started(_))))
    });
    let Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Started(summary))) = started else {
        panic!("not started");
    };
    assert_eq!(summary.kind, OpKind::Trash);
    let done = until(&mut rig.inbox, finished);
    let Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Finished(outcome))) = done else {
        panic!("not finished");
    };
    assert_eq!(outcome.finish, Finish::Done);
    assert_eq!(outcome.undo.map(|s| s.kind), Some(OpKind::Trash));
    assert!(!dir.path().join("notes.txt").exists());
    let rescans = [next(&mut rig.inbox), next(&mut rig.inbox)];
    for place in [Location::Folder(here), Location::Trash] {
        let want = Heard::All(Input::Changed(place, Change::Rescan));
        assert!(rescans.contains(&want), "{want:?} is not in {rescans:?}");
    }

    rig.edge.run_in(TabId::FIRST, vec![Effect::Undo]);
    assert_eq!(finish_of(until(&mut rig.inbox, finished)), Finish::Done);
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
    rig.edge.run_in(TabId::FIRST, vec![Effect::Run(plan)]);

    let asked = until(&mut rig.inbox, |h| {
        matches!(h, Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Asking(_))))
    });
    let Heard::Tab(TabId::FIRST, Input::Op(OpEvent::Asking(conflict))) = asked else {
        panic!("not asked");
    };
    assert_eq!(conflict.item, name("notes.txt"));
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Answer(Resolution::once(Answer::KeepBoth))],
    );
    assert_eq!(finish_of(until(&mut rig.inbox, finished)), Finish::Done);
    assert_eq!(
        std::fs::read_to_string(dir.path().join("notes 2.txt")).unwrap(),
        "other"
    );
}

#[test]
fn a_move_to_the_trash_is_announced_so_the_window_can_offer_to_undo_it() {
    let mut rig = rig();
    let summary = files_core::Summary::new(OpKind::Trash, 1, Some(name("x")));
    rig.edge
        .run_in(TabId::FIRST, vec![Effect::Announce(summary.clone())]);
    assert_eq!(next(&mut rig.inbox), Heard::Announced(summary));
}
