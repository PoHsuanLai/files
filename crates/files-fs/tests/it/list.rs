//! `DiskLister` against a scratch folder.

use std::collections::BTreeMap;
use std::fs;
use std::os::unix::fs::{PermissionsExt, symlink};
use std::os::unix::net::UnixListener;

use files_core::{ByteSize, EntryKind, FileName, FolderPath, LinkTo, Special, Visibility};
use files_fs::{DiskLister, ListError, Lister, Skipped};
use tempfile::TempDir;

fn folder(dir: &TempDir) -> FolderPath {
    FolderPath::new(dir.path()).unwrap()
}

fn name(s: &str) -> FileName {
    FileName::new(s).unwrap()
}

#[test]
fn lists_every_kind() {
    let dir = TempDir::new().unwrap();
    let p = dir.path();
    fs::write(p.join("file.txt"), "hello").unwrap();
    fs::write(p.join(".dot"), "").unwrap();
    fs::create_dir(p.join("sub")).unwrap();
    symlink(p.join("file.txt"), p.join("to-file")).unwrap();
    symlink(p.join("sub"), p.join("to-sub")).unwrap();
    symlink(p.join("missing"), p.join("broken")).unwrap();
    let _socket = UnixListener::bind(p.join("sock")).unwrap();

    let listing = DiskLister.list_counted(&folder(&dir)).unwrap();
    assert_eq!(listing.skipped, Skipped(0));
    let got: BTreeMap<_, _> = listing
        .entries
        .iter()
        .map(|e| (e.name.display().into_owned(), e.kind.clone()))
        .collect();

    let five = ByteSize(5);
    let want = [
        ("file.txt", EntryKind::File(five)),
        (".dot", EntryKind::File(ByteSize(0))),
        ("sub", EntryKind::Folder),
        ("to-file", EntryKind::Link(LinkTo::File(five))),
        ("to-sub", EntryKind::Link(LinkTo::Folder)),
        ("broken", EntryKind::Link(LinkTo::Broken)),
        ("sock", EntryKind::Special(Special::Socket)),
    ];
    assert_eq!(got.len(), want.len());
    for (n, kind) in want {
        assert_eq!(got.get(n), Some(&kind), "{n}");
    }
    let dot = listing.entries.iter().find(|e| e.name.display() == ".dot");
    assert_eq!(dot.unwrap().visibility(), Visibility::Hidden);
    assert!(
        listing
            .entries
            .iter()
            .all(|e| e.modified.is_some() && e.mime.is_none())
    );
}

#[test]
fn entry_matches_the_listing() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a"), "xyz").unwrap();
    let one = DiskLister.entry(&folder(&dir), &name("a")).unwrap();
    assert_eq!(one.kind, EntryKind::File(ByteSize(3)));
    let all = DiskLister.list(&folder(&dir)).unwrap();
    assert_eq!(all, vec![one]);
    assert_eq!(
        DiskLister.entry(&folder(&dir), &name("nope")),
        Err(ListError::NotFound)
    );
}

#[test]
fn errors() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("plain"), "").unwrap();
    let rows = [
        (dir.path().join("missing"), ListError::NotFound),
        (dir.path().join("plain"), ListError::NotAFolder),
    ];
    for (path, want) in rows {
        let got = DiskLister.list(&FolderPath::new(path.clone()).unwrap());
        assert_eq!(got, Err(want), "{}", path.display());
    }
}

#[test]
fn no_permission() {
    let dir = TempDir::new().unwrap();
    let locked = dir.path().join("locked");
    fs::create_dir(&locked).unwrap();
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o000)).unwrap();
    let readable = fs::read_dir(&locked).is_ok(); // running as root
    let got = DiskLister.list(&FolderPath::new(&locked).unwrap());
    fs::set_permissions(&locked, fs::Permissions::from_mode(0o755)).unwrap();
    if !readable {
        assert_eq!(got, Err(ListError::NoPermission));
    }
}

#[test]
fn error_words_are_plain() {
    assert_eq!(
        ListError::NotFound.to_string(),
        "The folder can't be found."
    );
}
