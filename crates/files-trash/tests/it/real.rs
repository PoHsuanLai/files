//! The real Trash, in a scratch data folder.
//!
//! The `trash` crate finds the home Trash from `XDG_DATA_HOME`, and setting an environment
//! variable in a running test is `unsafe`. So the real test starts this very test binary again as
//! a child process whose environment points at a scratch folder, and runs `child_round_trip`
//! there. Run by itself, `child_round_trip` does nothing: it needs the scratch folder named in
//! `FILES_TRASH_SCRATCH`. Neither test empties the Trash, which would reach other volumes' own
//! Trash folders; the child erases only the one item it threw away.

use std::fs;
use std::process::Command;

use files_core::{FileName, FolderPath, ItemRef};
use files_trash::{SystemTrash, Trash, TrashError};

const SCRATCH: &str = "FILES_TRASH_SCRATCH";

#[test]
fn the_real_trash_round_trips_in_a_scratch_data_folder() {
    let scratch = tempfile::tempdir().unwrap();
    fs::create_dir(scratch.path().join("data")).unwrap();
    fs::create_dir(scratch.path().join("home")).unwrap();
    let output = Command::new(std::env::current_exe().unwrap())
        .args([
            "--exact",
            "real::child_round_trip",
            "--nocapture",
            "--test-threads=1",
        ])
        .env("XDG_DATA_HOME", scratch.path().join("data"))
        .env("HOME", scratch.path().join("home"))
        .env(SCRATCH, scratch.path())
        .output()
        .unwrap();
    let said = String::from_utf8_lossy(&output.stdout).into_owned()
        + &String::from_utf8_lossy(&output.stderr);
    assert!(output.status.success(), "{said}");
    assert!(
        said.contains("1 passed"),
        "the child test did not run:\n{said}"
    );
}

/// The body of the real test; it only acts inside the child process.
#[test]
fn child_round_trip() {
    let Some(scratch) = std::env::var_os(SCRATCH) else {
        return;
    };
    let work = std::path::Path::new(&scratch).join("work");
    fs::create_dir(&work).unwrap();
    let item = ItemRef::new(
        FolderPath::new(&work).unwrap(),
        FileName::new("a.txt").unwrap(),
    );
    fs::write(item.path(), "hello").unwrap();

    let trash = SystemTrash;
    let thrown = trash.trash(std::slice::from_ref(&item)).unwrap();
    assert_eq!(thrown.len(), 1);
    assert_eq!(thrown[0].original, item);
    assert!(!item.path().exists());
    assert!(
        trash
            .list()
            .unwrap()
            .iter()
            .any(|held| held.id == thrown[0].id)
    );

    // Something is in its place now: refused, and nothing moves.
    fs::write(item.path(), "new").unwrap();
    assert_eq!(trash.put_back(&thrown), Err(TrashError::Occupied));
    fs::remove_file(item.path()).unwrap();

    trash.put_back(&thrown).unwrap();
    assert_eq!(fs::read_to_string(item.path()).unwrap(), "hello");

    let again = trash.trash(std::slice::from_ref(&item)).unwrap();
    trash.delete_for_good(&again).unwrap();
    assert!(
        !trash
            .list()
            .unwrap()
            .iter()
            .any(|held| held.id == again[0].id)
    );
}
