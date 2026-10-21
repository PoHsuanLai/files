//! The undo journal: each kind of work taken back, and the put-back round trip.

use files_core::{Answer, OpFailure, OpPlan, Resolution};
use files_ops::{Finish, Journal};
use files_trash::Trash;

use super::support::{Rig, Script, name, rig};

/// Runs `plan`, records it, and returns how it ended.
fn run(rig: &mut Rig, plan: &OpPlan) -> Finish {
    let ran = rig.exec.run(plan, &mut Script::default());
    rig.journal.record(&ran);
    ran.finish
}

fn undo(rig: &mut Rig) -> Finish {
    rig.journal
        .undo(&rig.exec, &mut Script::default())
        .unwrap()
        .finish
}

#[test]
fn a_copy_is_undone_by_trashing_the_copies() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    rig.write("home/tree/b.txt", "beta");
    let plan = OpPlan::Copy(rig.transfer(&["home/a.txt", "home/tree"]));
    run(&mut rig, &plan);
    assert_eq!(rig.names("dest"), ["a.txt", "tree"]);

    assert_eq!(undo(&mut rig), Finish::Done);
    assert!(rig.names("dest").is_empty());
    assert_eq!(
        rig.read("home/a.txt"),
        "alpha",
        "the originals are untouched"
    );
    assert_eq!(
        rig.trash.list().unwrap().len(),
        2,
        "the copies are in the Trash"
    );
}

#[test]
fn a_move_is_undone_by_moving_back() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    let plan = OpPlan::Move(rig.transfer(&["home/a.txt"]));
    run(&mut rig, &plan);
    assert!(!rig.exists("home/a.txt"));

    assert_eq!(undo(&mut rig), Finish::Done);
    assert_eq!(rig.read("home/a.txt"), "alpha");
    assert!(rig.names("dest").is_empty());
}

#[test]
fn a_new_folder_is_undone_only_while_it_is_empty() {
    let mut rig = rig();
    let plan = OpPlan::NewFolder {
        parent: rig.folder("home"),
        name: name("untitled folder"),
    };
    run(&mut rig, &plan);
    assert_eq!(undo(&mut rig), Finish::Done);
    assert!(rig.names("home").is_empty());

    run(&mut rig, &plan);
    rig.write("home/untitled folder/kept.txt", "k");
    assert_eq!(undo(&mut rig), Finish::Failed(OpFailure::NotEmpty));
    assert_eq!(rig.read("home/untitled folder/kept.txt"), "k");
}

#[test]
fn a_rename_is_undone_by_renaming_back() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    let plan = OpPlan::Rename {
        item: rig.item("home/a.txt"),
        to: name("b.txt"),
    };
    run(&mut rig, &plan);
    assert_eq!(rig.names("home"), ["b.txt"]);
    assert_eq!(undo(&mut rig), Finish::Done);
    assert_eq!(rig.names("home"), ["a.txt"]);
}

#[test]
fn a_move_to_the_trash_is_undone_by_putting_back() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    rig.write("home/b.txt", "beta");
    let plan = OpPlan::Trash {
        items: vec![rig.item("home/a.txt"), rig.item("home/b.txt")],
    };
    assert_eq!(run(&mut rig, &plan), Finish::Done);
    assert!(rig.names("home").is_empty());
    assert_eq!(rig.trash.list().unwrap().len(), 2);

    assert_eq!(undo(&mut rig), Finish::Done);
    assert_eq!(rig.names("home"), ["a.txt", "b.txt"]);
    assert!(rig.trash.list().unwrap().is_empty());
}

#[test]
fn put_back_round_trips_and_is_refused_when_the_place_is_taken() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    let trashed = rig.trash.trash(&[rig.item("home/a.txt")]).unwrap();
    let put_back = OpPlan::PutBack {
        items: trashed.clone(),
    };

    rig.write("home/a.txt", "in the way");
    assert_eq!(
        run(&mut rig, &put_back),
        Finish::Failed(OpFailure::Occupied)
    );
    assert_eq!(rig.read("home/a.txt"), "in the way");
    assert!(
        rig.journal.is_empty(),
        "nothing was done, so nothing to undo"
    );

    std::fs::remove_file(rig.path("home/a.txt")).unwrap();
    assert_eq!(run(&mut rig, &put_back), Finish::Done);
    assert_eq!(rig.read("home/a.txt"), "alpha");

    // And that is undone by trashing it again.
    assert_eq!(undo(&mut rig), Finish::Done);
    assert!(!rig.exists("home/a.txt"));
    assert_eq!(rig.trash.list().unwrap().len(), 1);
}

#[test]
fn erasing_for_good_cannot_be_undone() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    let trashed = rig.trash.trash(&[rig.item("home/a.txt")]).unwrap();
    let plan = OpPlan::DeleteForGood { items: trashed };
    run(&mut rig, &plan);
    assert!(rig.trash.list().unwrap().is_empty());
    assert!(rig.journal.is_empty());

    rig.write("home/b.txt", "beta");
    rig.trash.trash(&[rig.item("home/b.txt")]).unwrap();
    let plan = OpPlan::EmptyTrash;
    run(&mut rig, &plan);
    assert!(rig.trash.list().unwrap().is_empty());
    assert!(rig.journal.is_empty());
}

#[test]
fn undoing_a_replace_brings_back_both_the_old_item_and_the_empty_place() {
    let mut rig = rig();
    rig.write("home/a.txt", "new");
    rig.write("dest/a.txt", "old");
    let plan = OpPlan::Move(rig.transfer(&["home/a.txt"]));
    let mut host = Script::answering(&[Resolution::once(Answer::Replace)]);
    let ran = rig.exec.run(&plan, &mut host);
    rig.journal.record(&ran);
    assert_eq!(rig.read("dest/a.txt"), "new");

    assert_eq!(undo(&mut rig), Finish::Done);
    assert_eq!(rig.read("home/a.txt"), "new");
    assert_eq!(rig.read("dest/a.txt"), "old");
}

#[test]
fn the_newest_work_is_undone_first_and_an_empty_journal_undoes_nothing() {
    let mut rig = rig();
    rig.write("home/a.txt", "alpha");
    rig.write("home/b.txt", "beta");
    let plan = OpPlan::Move(rig.transfer(&["home/a.txt"]));
    run(&mut rig, &plan);
    let plan = OpPlan::Move(rig.transfer(&["home/b.txt"]));
    run(&mut rig, &plan);
    assert_eq!(rig.journal.len(), 2);

    undo(&mut rig);
    assert_eq!(rig.names("home"), ["b.txt"]);
    undo(&mut rig);
    assert_eq!(rig.names("home"), ["a.txt", "b.txt"]);

    let mut empty = Journal::new();
    assert!(empty.undo(&rig.exec, &mut Script::default()).is_none());
}
