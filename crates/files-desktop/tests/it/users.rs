//! Finding the programs that have a disk open, over a made-up process folder.

use std::os::unix::fs::symlink;
use std::path::Path;

use files_core::ProgramName;
use files_desktop::{DiskUsers, ProcUsers};

/// A process folder `pid` with its name, its working folder and the files it has open.
fn process(root: &Path, pid: &str, name: &str, cwd: &str, open: &[&str]) {
    let dir = root.join(pid);
    std::fs::create_dir_all(dir.join("fd")).unwrap();
    std::fs::write(dir.join("comm"), format!("{name}\n")).unwrap();
    symlink(cwd, dir.join("cwd")).unwrap();
    for (n, target) in open.iter().enumerate() {
        symlink(target, dir.join("fd").join(n.to_string())).unwrap();
    }
}

#[test]
fn the_programs_with_a_file_open_or_working_there_are_found_once_each() {
    let root = tempfile::tempdir().unwrap();
    process(
        root.path(),
        "10",
        "vlc",
        "/home/p",
        &["/run/media/p/STICK/film.mkv"],
    );
    process(
        root.path(),
        "11",
        "vlc",
        "/home/p",
        &["/run/media/p/STICK/other.mkv"],
    );
    process(root.path(), "12", "bash", "/run/media/p/STICK/dir", &[]);
    process(
        root.path(),
        "13",
        "firefox",
        "/home/p",
        &["/run/media/p/STICK2/x"],
    );
    process(root.path(), "14", "idle", "/", &["/dev/null", "socket:[1]"]);
    // Not a process: nothing to read.
    std::fs::write(root.path().join("version"), "x").unwrap();

    let users = ProcUsers::new(root.path());
    let found = users.using(Path::new("/run/media/p/STICK"));
    assert_eq!(
        found,
        vec![ProgramName::new("bash"), ProgramName::new("vlc")]
    );
    assert_eq!(users.using(Path::new("/run/media/p/NOTHING")), vec![]);
}

#[test]
fn a_missing_process_folder_finds_nobody() {
    let users = ProcUsers::new("/nonexistent/proc");
    assert_eq!(users.using(Path::new("/mnt")), vec![]);
}
