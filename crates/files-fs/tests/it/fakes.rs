//! The fakes behave as the traits promise.

use std::sync::mpsc;

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath};
use files_fs::{FakeLister, FakeWatcher, FolderEvent, ListError, Lister, Watcher};

fn folder(s: &str) -> FolderPath {
    FolderPath::new(s).unwrap()
}

fn file(s: &str) -> Entry {
    Entry::new(FileName::new(s).unwrap(), EntryKind::File(ByteSize(1)))
}

#[test]
fn fake_lister_answers_from_its_map() {
    let lister = FakeLister::new();
    assert_eq!(lister.list(&folder("/a")), Err(ListError::NotFound));
    lister.set(folder("/a"), Ok(vec![file("x")]));
    lister.set(folder("/b"), Err(ListError::NoPermission));
    assert_eq!(lister.list(&folder("/a")), Ok(vec![file("x")]));
    assert_eq!(lister.list(&folder("/b")), Err(ListError::NoPermission));
    let x = FileName::new("x").unwrap();
    assert_eq!(lister.entry(&folder("/a"), &x), Ok(file("x")));
    let y = FileName::new("y").unwrap();
    assert_eq!(lister.entry(&folder("/a"), &y), Err(ListError::NotFound));
}

#[test]
fn fake_watcher_delivers_until_dropped() {
    let watcher = FakeWatcher::new();
    let (tx, rx) = mpsc::channel();
    let handle = watcher
        .watch(&folder("/a"), Box::new(move |e| drop(tx.send(e))))
        .unwrap();
    assert_eq!(watcher.watched(), vec![folder("/a")]);
    watcher.push(&folder("/other"), FolderEvent::Rescan);
    watcher.push(&folder("/a"), FolderEvent::Gone);
    assert_eq!(rx.try_recv(), Ok(FolderEvent::Gone));
    assert!(rx.try_recv().is_err());
    drop(handle);
    assert!(watcher.watched().is_empty());
}
