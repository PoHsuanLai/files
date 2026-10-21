//! Tags in the window: the menu tags a file and its dot shows, and a tag place lists the files
//! with its colour.

use std::sync::Arc;
use std::time::Duration;

use ds::base::press::PointerButton;
use ds::prelude::ShortcutKey;
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_core::{FileName, FolderPath, ItemRef, TagColour, Tags};
use files_fs::{FakePrefs, FakeTagger, Tagger};
use files_trash::FakeTrash;
use files_ui::{PlaceKind, Places};
use tempfile::TempDir;

use super::support::{RecordingOpener, at, harness, launch_tagging, scratch_folder};

/// A window on a scratch folder as home, tags kept in `tagger`.
struct Rig {
    harness: Harness,
    dir: TempDir,
    _config: TempDir,
}

/// A window on `dir` as home, once it lists `last`.
fn open_in(dir: TempDir, tagger: &Arc<FakeTagger>, last: &'static str) -> Rig {
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let home = FolderPath::new(dir.path()).unwrap();
    let launch = launch_tagging(
        at(dir.path()),
        config.path(),
        &opener,
        trash,
        Arc::new(FakePrefs::new()),
        Arc::clone(tagger),
    )
    .with_home(home.clone())
    .with_places(Places::default().with(PlaceKind::Home, home));
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains(last));
    Rig {
        harness,
        dir,
        _config: config,
    }
}

fn open(tagger: &Arc<FakeTagger>) -> Rig {
    open_in(scratch_folder(), tagger, "zebra.png")
}

fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

/// The item at `parts` under the scratch folder.
fn item(dir: &TempDir, parts: &[&str]) -> ItemRef {
    let mut folder = FolderPath::new(dir.path()).unwrap();
    for part in &parts[..parts.len() - 1] {
        folder = folder.join(&name(part));
    }
    ItemRef::new(folder, name(parts[parts.len() - 1]))
}

/// The seven dots of the sidebar's Tags section.
const SIDEBAR_DOTS: usize = 7;

#[test]
fn the_sidebar_lists_the_seven_colours_under_tags_each_with_its_dot() {
    let rig = open(&Arc::new(FakeTagger::new()));
    let html = rig.harness.html();
    for text in [
        "Tags", "Red", "Orange", "Yellow", "Green", "Blue", "Purple", "Gray",
    ] {
        assert!(html.contains(text), "{text} is not in the sidebar:\n{html}");
    }
    assert_eq!(rig.harness.count(".ds-chip-dot"), SIDEBAR_DOTS);
}

#[test]
fn a_tagged_file_shows_a_dot_for_a_colour_and_a_chip_for_any_other_tag() {
    let tagger = Arc::new(FakeTagger::new());
    let dir = scratch_folder();
    tagger.put(item(&dir, &["notes.txt"]), Tags::parse("Red,Work"));
    let rig = open_in(dir, &tagger, "zebra.png");
    assert_eq!(rig.harness.count(".ds-chip-dot"), SIDEBAR_DOTS + 1);
    assert!(rig.harness.html().contains("Work"));
}

/// The context menu on row `n`, with the Tags submenu open and its first colour under the keys.
fn tags_menu(rig: &mut Rig, n: usize) {
    let selector = format!(".ds-vtable-item[*|data-row=\"{n}\"]");
    let point = rig.harness.centre(&selector).unwrap();
    rig.harness
        .send(Input::press(point, PointerButton::Secondary));
    settle_until(&mut rig.harness, |h| h.html().contains("Move to Trash"));
    // Open, Open With, Move to Trash, Get Info, Rename, Quick Look, Copy, Copy as Pathname, Tags.
    for _ in 0..9 {
        rig.harness.send(Input::key(ShortcutKey::Down));
        rig.harness.advance(Duration::from_millis(50));
    }
    rig.harness.send(Input::key(ShortcutKey::Right));
    settle_until(&mut rig.harness, |h| h.count(".ds-menu-item") > 12);
}

#[test]
fn tagging_a_file_from_the_menu_shows_its_dot() {
    let tagger = Arc::new(FakeTagger::new());
    let mut rig = open(&tagger);
    // The first colour is Red.
    tags_menu(&mut rig, 0);
    rig.harness.send(Input::key(ShortcutKey::Enter));
    settle_until(&mut rig.harness, |h| {
        h.count(".ds-chip-dot") == SIDEBAR_DOTS + 1
    });
    let notes = item(&rig.dir, &["notes.txt"]);
    assert!(tagger.tags(&notes).has(TagColour::Red));
}

#[test]
fn picking_the_colour_again_takes_the_tag_off() {
    let tagger = Arc::new(FakeTagger::new());
    let dir = scratch_folder();
    let notes = item(&dir, &["notes.txt"]);
    tagger.put(notes.clone(), Tags::parse("Red"));
    let mut rig = open_in(dir, &tagger, "zebra.png");
    assert_eq!(rig.harness.count(".ds-chip-dot"), SIDEBAR_DOTS + 1);
    tags_menu(&mut rig, 0);
    rig.harness.send(Input::key(ShortcutKey::Enter));
    settle_until(&mut rig.harness, |h| {
        h.count(".ds-chip-dot") == SIDEBAR_DOTS
    });
    assert!(!tagger.tags(&notes).has(TagColour::Red));
}

#[test]
fn a_tag_place_lists_the_tagged_files_under_home_with_their_own_names() {
    let tagger = Arc::new(FakeTagger::new());
    let dir = scratch_folder();
    // Two files of one name in different folders, both Red, and a Blue one.
    std::fs::write(dir.path().join("Projects").join("notes.txt"), "p").unwrap();
    tagger.put(item(&dir, &["notes.txt"]), Tags::parse("Red"));
    tagger.put(
        item(&dir, &["Projects", "notes.txt"]),
        Tags::parse("Red,Work"),
    );
    tagger.put(item(&dir, &["Projects", "inner.txt"]), Tags::parse("Blue"));
    let mut rig = open_in(dir, &tagger, "zebra.png");

    // The first row of the sidebar that leads with a dot is Red.
    let red = rig
        .harness
        .centre(".ds-sidebar .ds-row[*|data-leading=\"element\"]")
        .unwrap_or_else(|| panic!("no Red row:\n{}", rig.harness.html()));
    rig.harness.send(Input::click(red));
    rig.harness.advance(Duration::from_millis(50));
    settle_until(&mut rig.harness, |h| {
        h.html().matches("notes.txt").count() >= 2 && !h.html().contains("zebra.png")
    });
    let html = rig.harness.html();
    assert!(!html.contains("inner.txt"), "a Blue file is listed");
    // The walk is over: no bar turns.
    settle_until(&mut rig.harness, |h| h.count(".ds-progress") == 0);
}
