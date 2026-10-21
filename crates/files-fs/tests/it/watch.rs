//! `NotifyWatcher` against a scratch folder. Every wait is bounded.

use std::collections::HashSet;
use std::fs;
use std::sync::mpsc::{self, Receiver};
use std::time::{Duration, Instant};

use files_core::{FileName, FolderPath};
use files_fs::{FolderEvent, NotifyWatcher, WatchError, WatchHandle, Watcher};
use tempfile::TempDir;

const LIMIT: Duration = Duration::from_secs(5);

fn name(s: &str) -> FileName {
    FileName::new(s).unwrap()
}

fn follow(dir: &std::path::Path) -> (WatchHandle, Receiver<FolderEvent>) {
    let (tx, rx) = mpsc::channel();
    let handle = NotifyWatcher
        .watch(
            &FolderPath::new(dir).unwrap(),
            Box::new(move |e| drop(tx.send(e))),
        )
        .unwrap();
    (handle, rx)
}

/// Waits until every wanted event has been seen, or fails after the limit.
fn expect(rx: &Receiver<FolderEvent>, want: &[FolderEvent]) {
    let deadline = Instant::now() + LIMIT;
    let mut missing: HashSet<usize> = (0..want.len()).collect();
    let mut seen = Vec::new();
    while !missing.is_empty() {
        let left = deadline.saturating_duration_since(Instant::now());
        match rx.recv_timeout(left) {
            Ok(event) => {
                missing.retain(|i| want[*i] != event);
                seen.push(event);
            }
            Err(_) => panic!("wanted {want:?}, saw only {seen:?}"),
        }
    }
}

#[test]
fn create_and_delete() {
    let dir = TempDir::new().unwrap();
    let (_handle, rx) = follow(dir.path());
    fs::write(dir.path().join("a"), "x").unwrap();
    expect(&rx, &[FolderEvent::Added(name("a"))]);
    fs::remove_file(dir.path().join("a")).unwrap();
    expect(&rx, &[FolderEvent::Removed(name("a"))]);
}

#[test]
fn rename_within() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a"), "x").unwrap();
    let (_handle, rx) = follow(dir.path());
    fs::rename(dir.path().join("a"), dir.path().join("b")).unwrap();
    expect(
        &rx,
        &[
            FolderEvent::Removed(name("a")),
            FolderEvent::Added(name("b")),
        ],
    );
}

#[test]
fn rename_out() {
    let dir = TempDir::new().unwrap();
    let elsewhere = TempDir::new().unwrap();
    fs::write(dir.path().join("a"), "x").unwrap();
    let (_handle, rx) = follow(dir.path());
    fs::rename(dir.path().join("a"), elsewhere.path().join("a")).unwrap();
    expect(&rx, &[FolderEvent::Removed(name("a"))]);
}

#[test]
fn folder_removed() {
    let dir = TempDir::new().unwrap();
    let inner = dir.path().join("inner");
    fs::create_dir(&inner).unwrap();
    let (_handle, rx) = follow(&inner);
    fs::remove_dir(&inner).unwrap();
    expect(&rx, &[FolderEvent::Gone]);
}

#[test]
fn missing_folder_is_refused() {
    let dir = TempDir::new().unwrap();
    let result = NotifyWatcher.watch(
        &FolderPath::new(dir.path().join("missing")).unwrap(),
        Box::new(|_| {}),
    );
    assert_eq!(result.err(), Some(WatchError::NotFound));
}
