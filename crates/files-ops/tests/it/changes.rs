//! What each kind of plan does to the disk.

use std::fs;
use std::os::unix::fs::symlink;

use files_core::{OpFailure, OpPlan};
use files_ops::Finish;

use super::support::{Script, is_link, name, rig};

#[test]
fn a_copy_makes_whole_copies_and_leaves_the_originals() {
    let rig = rig();
    rig.write("home/a.txt", "alpha");
    rig.write("home/tree/deep/b.txt", "beta");
    let plan = OpPlan::Copy(rig.transfer(&["home/a.txt", "home/tree"]));
    let ran = rig.exec.run(&plan, &mut Script::default());

    assert_eq!(ran.finish, Finish::Done);
    assert_eq!(rig.read("dest/a.txt"), "alpha");
    assert_eq!(rig.read("dest/tree/deep/b.txt"), "beta");
    assert_eq!(rig.read("home/a.txt"), "alpha");
    assert!(ran.is_undoable());
}

#[test]
fn a_copy_copies_a_link_and_never_follows_it() {
    let rig = rig();
    rig.write("home/real/inside.txt", "x");
    symlink("real", rig.path("home/link")).unwrap();
    symlink("nowhere", rig.path("home/broken")).unwrap();
    let plan = OpPlan::Copy(rig.transfer(&["home/link", "home/broken"]));
    let ran = rig.exec.run(&plan, &mut Script::default());

    assert_eq!(ran.finish, Finish::Done);
    assert!(is_link(&rig.path("dest/link")));
    assert!(is_link(&rig.path("dest/broken")));
    assert_eq!(
        fs::read_link(rig.path("dest/link")).unwrap().to_str(),
        Some("real")
    );
    assert!(!rig.exists("dest/real"), "the link's target was not copied");
}

#[test]
fn a_copy_keeps_the_modification_time_and_reports_bytes() {
    let rig = rig();
    rig.write("home/a.txt", "12345");
    let before = fs::metadata(rig.path("home/a.txt"))
        .unwrap()
        .modified()
        .unwrap();
    let mut host = Script::default();
    rig.exec
        .run(&OpPlan::Copy(rig.transfer(&["home/a.txt"])), &mut host);
    let after = fs::metadata(rig.path("dest/a.txt"))
        .unwrap()
        .modified()
        .unwrap();
    assert_eq!(before, after);
    let last = host.progress.last().unwrap();
    assert_eq!((last.items_done, last.items_total), (1, 1));
    assert_eq!((last.bytes_done.0, last.bytes_total.0), (5, 5));
    assert_eq!(last.permille(), 1000);
}

#[test]
fn a_move_on_one_disk_is_a_rename() {
    let rig = rig();
    rig.write("home/a.txt", "alpha");
    let inode = |p: &str| std::os::unix::fs::MetadataExt::ino(&fs::metadata(rig.path(p)).unwrap());
    let before = inode("home/a.txt");
    let ran = rig.exec.run(
        &OpPlan::Move(rig.transfer(&["home/a.txt"])),
        &mut Script::default(),
    );
    assert_eq!(ran.finish, Finish::Done);
    assert!(!rig.exists("home/a.txt"));
    assert_eq!(inode("dest/a.txt"), before);
}

#[test]
fn a_drag_moves_on_one_disk() {
    let rig = rig();
    rig.write("home/a.txt", "alpha");
    let ran = rig.exec.run(
        &OpPlan::Drag(rig.transfer(&["home/a.txt"])),
        &mut Script::default(),
    );
    assert_eq!(ran.finish, Finish::Done);
    assert!(!rig.exists("home/a.txt"));
    assert_eq!(rig.read("dest/a.txt"), "alpha");
}

#[test]
fn moving_an_item_into_its_own_folder_does_nothing() {
    let rig = rig();
    rig.write("dest/a.txt", "alpha");
    let ran = rig.exec.run(
        &OpPlan::Move(rig.transfer(&["dest/a.txt"])),
        &mut Script::default(),
    );
    assert_eq!(ran.finish, Finish::Done);
    assert_eq!(rig.names("dest"), ["a.txt"]);
    assert!(!ran.is_undoable());
}

#[test]
fn copying_into_its_own_folder_makes_a_second_one_without_asking() {
    let rig = rig();
    rig.write("dest/a.txt", "alpha");
    let mut host = Script::default();
    rig.exec
        .run(&OpPlan::Copy(rig.transfer(&["dest/a.txt"])), &mut host);
    assert!(host.asked.is_empty());
    assert_eq!(rig.names("dest"), ["a 2.txt", "a.txt"]);
}

#[test]
fn a_folder_cannot_be_put_inside_itself() {
    let rig = rig();
    rig.write("home/tree/leaf", "x");
    let inside = rig.folder("home/tree");
    let plan = OpPlan::Move(files_core::Transfer::new(
        vec![rig.item("home/tree")],
        inside,
    ));
    let ran = rig.exec.run(&plan, &mut Script::default());
    assert_eq!(ran.finish, Finish::Failed(OpFailure::IntoItself));
    assert!(rig.exists("home/tree/leaf"));
}

#[test]
fn a_new_folder_is_made_and_a_taken_name_is_refused() {
    let rig = rig();
    let plan = OpPlan::NewFolder {
        parent: rig.folder("home"),
        name: name("untitled folder"),
    };
    assert_eq!(
        rig.exec.run(&plan, &mut Script::default()).finish,
        Finish::Done
    );
    assert!(rig.path("home/untitled folder").is_dir());
    assert_eq!(
        rig.exec.run(&plan, &mut Script::default()).finish,
        Finish::Failed(OpFailure::Taken)
    );
}

#[test]
fn a_rename_never_replaces() {
    let rig = rig();
    rig.write("home/a.txt", "a");
    rig.write("home/b.txt", "b");
    let rename = |to: &str| OpPlan::Rename {
        item: rig.item("home/a.txt"),
        to: name(to),
    };
    let taken = rig.exec.run(&rename("b.txt"), &mut Script::default());
    assert_eq!(taken.finish, Finish::Failed(OpFailure::Taken));
    assert_eq!(rig.read("home/b.txt"), "b");

    let free = rig.exec.run(&rename("c.txt"), &mut Script::default());
    assert_eq!(free.finish, Finish::Done);
    assert_eq!(rig.names("home"), ["b.txt", "c.txt"]);
}

#[test]
fn a_missing_source_fails_as_not_found_and_stops_there() {
    let rig = rig();
    rig.write("home/a.txt", "a");
    let ghost = files_core::ItemRef::new(rig.folder("home"), name("ghost"));
    let mut sources = vec![rig.item("home/a.txt"), ghost];
    sources.push(rig.item("home/a.txt"));
    let plan = OpPlan::Copy(files_core::Transfer::new(sources, rig.folder("dest")));
    let ran = rig.exec.run(&plan, &mut Script::default());
    assert_eq!(ran.finish, Finish::Failed(OpFailure::NotFound));
    assert!(rig.exists("dest/a.txt"), "the first item was done");
    assert!(ran.is_undoable(), "and can be taken back");
}

#[test]
fn a_cancelled_copy_stops_and_leaves_no_half_made_item() {
    let rig = rig();
    rig.write("home/one", "1");
    rig.write("home/two", "2");
    // Stop after the first progress report of the copy (the start), so nothing is copied.
    let mut host = Script::stopping_after(1);
    let plan = OpPlan::Copy(rig.transfer(&["home/one", "home/two"]));
    let ran = rig.exec.run(&plan, &mut host);
    assert_eq!(ran.finish, Finish::Stopped);
    assert!(rig.names("dest").is_empty());

    // Stop after the first item is done: that one stays, the next is not begun.
    let mut host = Script::stopping_after(3);
    let ran = rig.exec.run(&plan, &mut host);
    assert_eq!(ran.finish, Finish::Stopped);
    assert_eq!(rig.names("dest"), ["one"]);
}
