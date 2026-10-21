//! A name that is taken where an item is going: every answer, and "for the rest".

use files_core::{Answer, ConflictKind, OpPlan, Resolution};
use files_ops::Finish;

use super::support::{Script, name, rig};

/// `home/a.txt`, `home/b.txt` and `home/c.txt` hold "new a", "new b", "new c"; `dest/` already
/// holds a.txt and b.txt as "old a" and "old b".
fn rig_with_clashes() -> super::support::Rig {
    let rig = rig();
    for letter in ["a", "b", "c"] {
        rig.write(&format!("home/{letter}.txt"), &format!("new {letter}"));
    }
    for letter in ["a", "b"] {
        rig.write(&format!("dest/{letter}.txt"), &format!("old {letter}"));
    }
    rig
}

fn copy_all(rig: &super::support::Rig, host: &mut Script) -> Finish {
    let plan = OpPlan::Copy(rig.transfer(&["home/a.txt", "home/b.txt", "home/c.txt"]));
    rig.exec.run(&plan, host).finish
}

#[test]
fn keep_both_names_the_new_one_with_a_number() {
    let rig = rig_with_clashes();
    let mut host = Script::answering(&[Resolution::for_rest(Answer::KeepBoth)]);
    assert_eq!(copy_all(&rig, &mut host), Finish::Done);

    assert_eq!(host.asked.len(), 1, "for the rest: asked once");
    assert_eq!(
        rig.names("dest"),
        ["a 2.txt", "a.txt", "b 2.txt", "b.txt", "c.txt"]
    );
    assert_eq!(rig.read("dest/a.txt"), "old a");
    assert_eq!(rig.read("dest/a 2.txt"), "new a");
}

#[test]
fn the_question_says_what_is_taken_and_how_many_come_after() {
    let rig = rig_with_clashes();
    let mut host = Script::answering(&[
        Resolution::once(Answer::KeepBoth),
        Resolution::once(Answer::KeepBoth),
    ]);
    copy_all(&rig, &mut host);
    assert_eq!(host.asked.len(), 2);
    let first = &host.asked[0];
    assert_eq!(first.item, name("a.txt"));
    assert_eq!(first.folder, rig.folder("dest"));
    assert_eq!(first.kind, ConflictKind::Copy);
    assert_eq!(first.rest, 2);
    assert_eq!(host.asked[1].rest, 1);
}

#[test]
fn replace_puts_the_old_one_in_the_trash() {
    let rig = rig_with_clashes();
    let mut host = Script::answering(&[Resolution::for_rest(Answer::Replace)]);
    assert_eq!(copy_all(&rig, &mut host), Finish::Done);

    assert_eq!(rig.read("dest/a.txt"), "new a");
    assert_eq!(rig.read("dest/b.txt"), "new b");
    assert_eq!(rig.names("dest"), ["a.txt", "b.txt", "c.txt"]);
    use files_trash::Trash;
    assert_eq!(
        rig.trash.list().unwrap().len(),
        2,
        "both old ones are in the Trash"
    );
}

#[test]
fn skip_leaves_this_one_out_and_goes_on() {
    let rig = rig_with_clashes();
    let mut host = Script::answering(&[
        Resolution::once(Answer::Skip),
        Resolution::once(Answer::Replace),
    ]);
    assert_eq!(copy_all(&rig, &mut host), Finish::Done);
    assert_eq!(rig.read("dest/a.txt"), "old a");
    assert_eq!(rig.read("dest/b.txt"), "new b");
    assert_eq!(rig.read("dest/c.txt"), "new c");
}

#[test]
fn stop_ends_the_work_and_keeps_what_was_done() {
    let rig = rig_with_clashes();
    rig.write("dest/c.txt", "old c");
    std::fs::remove_file(rig.path("dest/a.txt")).unwrap();
    // a.txt goes in freely, then b.txt is taken and the answer is Stop.
    let mut host = Script::answering(&[Resolution::once(Answer::Stop)]);
    assert_eq!(copy_all(&rig, &mut host), Finish::Stopped);
    assert_eq!(rig.read("dest/a.txt"), "new a");
    assert_eq!(rig.read("dest/b.txt"), "old b");
    assert_eq!(rig.read("dest/c.txt"), "old c");
}

#[test]
fn a_move_asks_with_the_move_kind() {
    let rig = rig_with_clashes();
    let mut host = Script::answering(&[Resolution::once(Answer::Stop)]);
    let plan = OpPlan::Move(rig.transfer(&["home/a.txt"]));
    rig.exec.run(&plan, &mut host);
    assert_eq!(host.asked[0].kind, ConflictKind::Move);
    assert_eq!(rig.read("home/a.txt"), "new a", "nothing moved");
}
