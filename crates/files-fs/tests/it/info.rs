//! Get Info's facts, from the disk and from the fake.

use std::fs;

use files_core::{ByteSize, EntryKind, FileName, FolderPath, Info, ItemCount, ItemRef};
use files_fs::{DiskLister, FakeLister, ListError, Lister};
use tempfile::TempDir;

fn item(dir: &TempDir, name: &str) -> ItemRef {
    ItemRef::new(
        FolderPath::new(dir.path()).unwrap(),
        FileName::new(name).unwrap(),
    )
}

#[test]
fn a_file_has_its_size_and_dates() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "hello").unwrap();
    let info = DiskLister.info(&item(&dir, "a.txt")).unwrap();
    assert_eq!(info.kind, EntryKind::File(ByteSize(5)));
    assert_eq!(info.size(), Some(ByteSize(5)));
    assert!(info.modified.is_some());
    assert_eq!(info.items, None);
    assert_eq!(info.folder, FolderPath::new(dir.path()).unwrap());
}

#[test]
fn a_folder_counts_what_a_person_sees() {
    let dir = TempDir::new().unwrap();
    let inner = dir.path().join("inner");
    fs::create_dir(&inner).unwrap();
    for name in ["a", "b", ".hidden"] {
        fs::write(inner.join(name), "").unwrap();
    }
    let info = DiskLister.info(&item(&dir, "inner")).unwrap();
    assert_eq!(info.kind, EntryKind::Folder);
    assert_eq!(info.items, Some(ItemCount(2)));
}

#[test]
fn an_item_that_is_gone_is_not_found() {
    let dir = TempDir::new().unwrap();
    assert_eq!(
        DiskLister.info(&item(&dir, "nothing")),
        Err(ListError::NotFound)
    );
}

#[test]
fn the_fake_describes_what_it_was_given() {
    let dir = TempDir::new().unwrap();
    let folder = FolderPath::new(dir.path()).unwrap();
    let inner = ItemRef::new(folder.clone(), FileName::new("inner").unwrap());
    let fake = FakeLister::new();
    let entry = |name: &str, kind| files_core::Entry::new(FileName::new(name).unwrap(), kind);
    fake.set(
        folder,
        Ok(vec![
            entry("inner", EntryKind::Folder),
            entry("f", EntryKind::File(ByteSize(9))),
        ]),
    );
    fake.set(
        inner.as_folder(),
        Ok(vec![
            entry("x", EntryKind::Folder),
            entry(".h", EntryKind::Folder),
        ]),
    );
    let got: Info = fake.info(&inner).unwrap();
    assert_eq!(got.items, Some(ItemCount(1)));
    let file = ItemRef::new(inner.folder.clone(), FileName::new("f").unwrap());
    assert_eq!(fake.info(&file).unwrap().size(), Some(ByteSize(9)));
}
