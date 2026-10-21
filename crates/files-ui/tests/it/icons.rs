//! The icon view in the window on a scratch folder: switching to it, selecting and opening
//! tiles, renaming one, and the view being remembered for the folder.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_core::{FolderPath, FolderPrefs, ViewMode};
use files_fs::{FakePrefs, PrefsStore};
use files_trash::FakeTrash;
use tempfile::TempDir;

use super::support::{RecordingOpener, at, harness, launch_remembering, scratch_folder};

/// The tile `n`. By name the scratch folder sorts `notes.txt`, `Projects`, `report.pdf`, `zebra.png`.
fn tile(n: usize) -> String {
    format!(".ds-cv-item[*|data-index=\"{n}\"]")
}

/// The Icons segment of the toolbar's switcher, the last of its two.
const ICONS_SEGMENT: &str = ".ds-segmented-segment:last-of-type";

/// A window on `dir` over `prefs` with the opener it reports to, once it has listed everything.
fn open(dir: &Path, prefs: &Arc<FakePrefs>) -> (Harness, Arc<RecordingOpener>, TempDir) {
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let launch = launch_remembering(at(dir), config.path(), &opener, trash, Arc::clone(prefs));
    let mut window = harness(launch);
    settle_until(&mut window, |h| h.html().contains("zebra.png"));
    (window, opener, config)
}

fn command(window: &mut Harness, key: ShortcutKey) {
    window.send(Input::chord(&[ShortcutKey::Super], key));
    window.advance(Duration::from_millis(50));
}

fn click(window: &mut Harness, selector: &str) {
    let at: Point = window
        .centre(selector)
        .unwrap_or_else(|| panic!("no {selector}:\n{}", window.html()));
    window.send(Input::click(at));
    window.advance(Duration::from_millis(50));
}

fn selected(window: &Harness, n: usize) -> bool {
    window.attr(&tile(n), "aria-selected").as_deref() == Some("true")
}

/// A window on `dir` showing icons.
fn icons(dir: &Path) -> (Harness, Arc<RecordingOpener>, TempDir) {
    let (mut window, opener, config) = open(dir, &Arc::new(FakePrefs::new()));
    command(&mut window, ShortcutKey::Char('1'));
    settle_until(&mut window, |h| h.count(".ds-cv-item") > 0);
    (window, opener, config)
}

#[test]
fn the_switcher_and_the_keys_change_the_view_in_both_directions() {
    let dir = scratch_folder();
    let (mut window, _opener, _config) = open(dir.path(), &Arc::new(FakePrefs::new()));
    assert_eq!(window.count(".ds-cv-item"), 0);

    click(&mut window, ICONS_SEGMENT);
    settle_until(&mut window, |h| h.count(".ds-cv-item") == 4);
    assert_eq!(window.count(".ds-vtable"), 0, "the table is still drawn");
    assert!(window.html().contains("zebra.png"));

    command(&mut window, ShortcutKey::Char('2'));
    settle_until(&mut window, |h| h.count(".ds-cv-item") == 0);
    assert!(window.html().contains("Date Modified"));

    command(&mut window, ShortcutKey::Char('1'));
    settle_until(&mut window, |h| h.count(".ds-cv-item") == 4);
}

#[test]
fn a_press_selects_a_tile_and_another_replaces_it() {
    let dir = scratch_folder();
    let (mut window, _opener, _config) = icons(dir.path());
    click(&mut window, &tile(1));
    assert!(selected(&window, 1));
    click(&mut window, &tile(3));
    assert!(selected(&window, 3));
    assert!(!selected(&window, 1), "a plain click keeps the old tile");
}

#[test]
fn a_double_click_opens_a_folder_and_a_file_and_command_down_opens_the_selection() {
    let dir = scratch_folder();
    let (mut window, opener, _config) = icons(dir.path());
    // Two presses on one spot with no time between them are a double click.
    let at: Point = window.centre(&tile(2)).unwrap();
    window.send(Input::click(at));
    window.send(Input::click(at));
    window.advance(Duration::from_millis(50));
    assert_eq!(opener.opened(), vec![dir.path().join("report.pdf")]);

    click(&mut window, &tile(1));
    command(&mut window, ShortcutKey::Down);
    settle_until(&mut window, |h| h.html().contains("inner.txt"));
    command(&mut window, ShortcutKey::Up);
    settle_until(&mut window, |h| h.html().contains("zebra.png"));
}

#[test]
fn return_renames_a_tile_in_place_and_escape_puts_the_name_back() {
    let dir = scratch_folder();
    let (mut window, _opener, _config) = icons(dir.path());
    click(&mut window, &tile(0));
    window.send(Input::key(ShortcutKey::Enter));
    settle_until(&mut window, |h| {
        h.count(".ds-cv-item[*|data-editing] .ds-text-field") > 0
    });
    window.send(Input::key(ShortcutKey::Escape));
    settle_until(&mut window, |h| h.count(".ds-text-field") == 0);
    assert!(window.html().contains("notes.txt"));
}

#[test]
fn space_opens_quick_look_on_the_selected_tile() {
    let dir = scratch_folder();
    let (mut window, _opener, _config) = icons(dir.path());
    click(&mut window, &tile(0));
    window.send(Input::key(ShortcutKey::Space));
    settle_until(&mut window, |h| h.count(".ds-peek") > 0);
}

#[test]
fn the_view_is_remembered_for_the_folder_and_not_for_another() {
    let dir = scratch_folder();
    let prefs = Arc::new(FakePrefs::new());
    let (mut first, _opener, _config) = open(dir.path(), &prefs);
    command(&mut first, ShortcutKey::Char('1'));
    settle_until(&mut first, |h| h.count(".ds-cv-item") > 0);
    // The store is written from a thread of its own: wait for the write before the "quit".
    let folder = FolderPath::new(dir.path()).unwrap();
    let started = std::time::Instant::now();
    while prefs.load(&folder).view != ViewMode::Icons {
        assert!(started.elapsed() < Duration::from_secs(5), "never saved");
        std::thread::sleep(Duration::from_millis(5));
    }
    drop(first);

    let (mut second, _opener, _again) = open(dir.path(), &prefs);
    settle_until(&mut second, |h| h.count(".ds-cv-item") > 0);

    let other = scratch_folder();
    let other_folder = FolderPath::new(other.path()).unwrap();
    assert_eq!(prefs.load(&other_folder), FolderPrefs::default());
    let (third, _opener, _third) = open(other.path(), &prefs);
    assert_eq!(third.count(".ds-cv-item"), 0, "the list is the default");
}
