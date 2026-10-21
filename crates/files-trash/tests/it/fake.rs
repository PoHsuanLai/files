//! The Trash contract, proved on `FakeTrash` over a scratch folder.

use std::fs;

use files_core::{FileName, FolderPath, ItemRef};
use files_trash::{FakeTrash, Trash, TrashError};
use tempfile::TempDir;

/// A scratch folder with `home/` for the files and `bin/` for the fake Trash.
struct Rig {
    dir: TempDir,
    trash: FakeTrash,
}

fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("home")).unwrap();
    let trash = FakeTrash::new(dir.path().join("bin"));
    Rig { dir, trash }
}

impl Rig {
    fn item(&self, name: &str) -> ItemRef {
        let folder = FolderPath::new(self.dir.path().join("home")).unwrap();
        ItemRef::new(folder, FileName::new(name).unwrap())
    }

    fn make(&self, name: &str, text: &str) -> ItemRef {
        let item = self.item(name);
        fs::write(item.path(), text).unwrap();
        item
    }
}

#[test]
fn a_trashed_file_leaves_and_comes_back_whole() {
    let rig = rig();
    let item = rig.make("a.txt", "hello");
    let thrown = rig.trash.trash(std::slice::from_ref(&item)).unwrap();
    assert_eq!(thrown.len(), 1);
    assert_eq!(thrown[0].original, item);
    assert!(!item.path().exists());
    assert_eq!(rig.trash.list().unwrap(), thrown);

    rig.trash.put_back(&thrown).unwrap();
    assert_eq!(fs::read_to_string(item.path()).unwrap(), "hello");
    assert!(rig.trash.list().unwrap().is_empty());
}

#[test]
fn a_put_back_into_an_occupied_place_is_refused_and_moves_nothing() {
    let rig = rig();
    let (a, b) = (rig.make("a.txt", "old a"), rig.make("b.txt", "old b"));
    let thrown = rig.trash.trash(&[a.clone(), b.clone()]).unwrap();
    rig.make("b.txt", "new b");

    assert_eq!(rig.trash.put_back(&thrown), Err(TrashError::Occupied));
    assert!(!a.path().exists(), "the free one stayed in the Trash");
    assert_eq!(fs::read_to_string(b.path()).unwrap(), "new b");
    assert_eq!(rig.trash.list().unwrap().len(), 2);
}

#[test]
fn two_items_of_one_name_are_both_kept_and_come_back_one_at_a_time() {
    let rig = rig();
    let item = rig.make("a.txt", "first");
    let first = rig.trash.trash(std::slice::from_ref(&item)).unwrap();
    rig.make("a.txt", "second");
    let second = rig.trash.trash(std::slice::from_ref(&item)).unwrap();

    let listed = rig.trash.list().unwrap();
    assert_eq!(listed.len(), 2);
    let stored: Vec<_> = listed
        .iter()
        .map(|i| i.stored_name().unwrap().to_string())
        .collect();
    assert_eq!(stored, ["a.txt", "a 2.txt"]);
    assert!(
        second[0].deleted > first[0].deleted,
        "the newest is the last thrown away"
    );

    rig.trash.put_back(&second).unwrap();
    assert_eq!(fs::read_to_string(item.path()).unwrap(), "second");
    assert_eq!(rig.trash.put_back(&first), Err(TrashError::Occupied));
}

#[test]
fn a_folder_is_trashed_with_what_is_in_it() {
    let rig = rig();
    let folder = rig.item("stuff");
    fs::create_dir(folder.path()).unwrap();
    fs::write(folder.path().join("inner"), "x").unwrap();
    let thrown = rig.trash.trash(std::slice::from_ref(&folder)).unwrap();
    assert_eq!(thrown[0].kind, files_core::EntryKind::Folder);
    rig.trash.put_back(&thrown).unwrap();
    assert!(folder.path().join("inner").exists());
}

#[test]
fn erasing_for_good_and_emptying_leave_nothing_to_put_back() {
    let rig = rig();
    let (a, b, c) = (rig.make("a", "1"), rig.make("b", "2"), rig.make("c", "3"));
    let thrown = rig.trash.trash(&[a, b, c]).unwrap();

    rig.trash.delete_for_good(&thrown[..1]).unwrap();
    assert_eq!(rig.trash.list().unwrap().len(), 2);
    assert_eq!(rig.trash.put_back(&thrown[..1]), Err(TrashError::NotFound));

    rig.trash.empty().unwrap();
    assert!(rig.trash.list().unwrap().is_empty());
    assert!(
        fs::read_dir(rig.dir.path().join("bin").join("files"))
            .unwrap()
            .next()
            .is_none()
    );
}

#[test]
fn trashing_what_is_not_there_says_so() {
    let rig = rig();
    assert_eq!(
        rig.trash.trash(&[rig.item("ghost")]),
        Err(TrashError::NotFound)
    );
}
