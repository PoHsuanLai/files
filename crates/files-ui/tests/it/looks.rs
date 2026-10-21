//! Quick Look, Get Info and the folder views that are remembered, in the window on a scratch
//! folder: the same harness `window.rs` uses.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_core::{FolderPath, FolderPrefs};
use files_fs::{FakePrefs, PrefsStore};
use files_trash::FakeTrash;
use tempfile::TempDir;

use super::support::{RecordingOpener, at, harness, launch_remembering, scratch_folder};

/// The table row `n`, the first of its two panes.
fn row(n: usize) -> String {
    format!(".ds-vtable-item[*|data-row=\"{n}\"]")
}

/// The header of the Size column: the second of the scrolling pane's (Date Modified, Size, Kind).
const SIZE_HEAD: &str = ".ds-vtable-head-track .ds-table-head:nth-child(2) > .ds-table-column";

/// A window on `dir` over `prefs`, once it has listed everything in it.
fn open(dir: &Path, prefs: &Arc<FakePrefs>) -> (Harness, TempDir) {
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let launch = launch_remembering(at(dir), config.path(), &opener, trash, Arc::clone(prefs));
    let mut window = harness(launch);
    settle_until(&mut window, |h| h.html().contains("zebra.png"));
    (window, config)
}

fn click_row(window: &mut Harness, n: usize) {
    let at: Point = window
        .centre(&row(n))
        .unwrap_or_else(|| panic!("no row {n}:\n{}", window.html()));
    window.send(Input::click(at));
    window.advance(Duration::from_millis(50));
}

fn press(window: &mut Harness, key: ShortcutKey) {
    window.send(Input::key(key));
    window.advance(Duration::from_millis(50));
}

fn quick_look_label(window: &Harness) -> Option<String> {
    window.attr(".ds-peek", "aria-label")
}

#[test]
fn space_opens_quick_look_on_the_selected_file_and_space_or_escape_closes_it() {
    let dir = scratch_folder();
    let (mut window, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    click_row(&mut window, 0);
    for closing in [ShortcutKey::Space, ShortcutKey::Escape] {
        press(&mut window, ShortcutKey::Space);
        settle_until(&mut window, |h| h.count(".ds-peek") > 0);
        assert_eq!(quick_look_label(&window).as_deref(), Some("notes.txt"));
        // The card anyview's worker made for the file.
        settle_until(&mut window, |h| h.count(".anyview-pane") > 0);
        press(&mut window, closing);
        settle_until(&mut window, |h| h.count(".ds-peek") == 0);
    }
}

#[test]
fn command_y_opens_quick_look_too() {
    let dir = scratch_folder();
    let (mut window, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    click_row(&mut window, 0);
    window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('y')));
    settle_until(&mut window, |h| h.count(".ds-peek") > 0);
}

#[test]
fn quick_look_follows_the_arrow_keys_and_shows_a_folder() {
    let dir = scratch_folder();
    let (mut window, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    click_row(&mut window, 0);
    press(&mut window, ShortcutKey::Space);
    settle_until(&mut window, |h| h.count(".ds-peek") > 0);
    press(&mut window, ShortcutKey::Down);
    settle_until(&mut window, |h| {
        h.attr(".ds-peek", "aria-label").as_deref() == Some("Projects")
    });
    // The folder: its icon, its name and what is in it.
    settle_until(&mut window, |h| h.count(".anyview-pane") > 0);
    press(&mut window, ShortcutKey::Down);
    settle_until(&mut window, |h| {
        h.attr(".ds-peek", "aria-label").as_deref() == Some("report.pdf")
    });
    assert_eq!(
        window.attr(&row(2), "data-selected").as_deref(),
        Some("yes"),
        "the selection moved with it"
    );
}

#[test]
fn space_with_nothing_selected_opens_nothing() {
    let dir = scratch_folder();
    let (mut window, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    press(&mut window, ShortcutKey::Space);
    assert_eq!(window.count(".ds-peek"), 0);
}

#[test]
fn command_i_opens_get_info_with_the_size_in_words_and_in_bytes() {
    let dir = scratch_folder();
    std::fs::write(dir.path().join("big.bin"), vec![0u8; 12_345]).unwrap();
    let (mut window, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    // By name: big.bin, notes.txt, Projects, report.pdf, zebra.png.
    click_row(&mut window, 0);
    window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('i')));
    settle_until(&mut window, |h| h.html().contains("12 KB (12,345 bytes)"));
    let html = window.html();
    for text in ["Kind", "BIN file", "Where", "Modified", "Extension"] {
        assert!(html.contains(text), "{text} is not shown:\n{html}");
    }
    let place = dir.path().display().to_string();
    assert!(html.contains(&place), "the folder is not named:\n{html}");
    press(&mut window, ShortcutKey::Escape);
    settle_until(&mut window, |h| h.count(".ds-sheet") == 0);
}

#[test]
fn get_info_on_a_folder_counts_what_is_in_it() {
    let dir = scratch_folder();
    let (mut window, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    click_row(&mut window, 1);
    window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('i')));
    settle_until(&mut window, |h| h.html().contains("1 item"));
    let sheet = window.text_of(".ds-sheet").unwrap_or_default();
    assert!(!sheet.contains("bytes"), "a folder has no size: {sheet}");
}

#[test]
fn a_sort_change_survives_a_relaunch_through_the_store() {
    let dir = scratch_folder();
    let prefs = Arc::new(FakePrefs::new());
    let (mut first, _config) = open(dir.path(), &prefs);
    assert_eq!(first.text_of(&row(0)).as_deref(), Some("notes.txt"));
    let size = first
        .centre(SIZE_HEAD)
        .unwrap_or_else(|| panic!("no Size header:\n{}", first.html()));
    first.send(Input::click(size));
    settle_until(&mut first, |h| {
        h.text_of(&row(0)).as_deref() != Some("notes.txt")
    });
    let sorted = first.text_of(&row(0));
    // The store is written from a thread of its own: wait for the write before the "quit".
    let folder = FolderPath::new(dir.path()).unwrap();
    let started = std::time::Instant::now();
    while prefs.load(&folder) == FolderPrefs::default() {
        assert!(started.elapsed() < Duration::from_secs(5), "never saved");
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(first);

    let (mut second, _again) = open(dir.path(), &prefs);
    settle_until(&mut second, |h| h.text_of(&row(0)) == sorted);
}
