//! The window in ds-harness: a real Blitz document with the app's root over a scratch folder,
//! driven by the events a window would deliver. The disk is a scratch folder's, the look is read
//! from a scratch config folder, and nothing opens a real program.

use std::sync::Arc;
use std::time::Duration;

use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_core::FolderPath;
use files_ui::{PlaceKind, Places};
use tempfile::TempDir;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

/// A window on a scratch folder, with what it needs kept alive.
struct Rig {
    harness: Harness,
    opener: Arc<RecordingOpener>,
    dir: TempDir,
    _config: TempDir,
}

/// The table row `n`, the first of its two panes.
fn row(n: usize) -> String {
    format!(".ds-vtable-item[*|data-row=\"{n}\"]")
}

/// A window on `dir` with `places` in its sidebar, once it has listed `last`.
fn open_with(dir: TempDir, places: Places, last: &str) -> Rig {
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let launch = launch(dir.path(), config.path(), &opener).with_places(places);
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains(last));
    Rig {
        harness,
        opener,
        dir,
        _config: config,
    }
}

/// A window on the scratch folder, once it has listed everything in it.
fn open() -> Rig {
    open_with(scratch_folder(), Places::default(), "zebra.png")
}

impl Rig {
    fn click_row(&mut self, n: usize) {
        let at: Point = self
            .harness
            .centre(&row(n))
            .unwrap_or_else(|| panic!("no row {n}:\n{}", self.harness.html()));
        self.harness.send(Input::click(at));
        self.harness.advance(Duration::from_millis(50));
    }

    fn selected(&self, n: usize) -> bool {
        self.harness.attr(&row(n), "data-selected").as_deref() == Some("yes")
    }

    fn command(&mut self, key: ShortcutKey) {
        self.harness.send(Input::chord(&[ShortcutKey::Super], key));
        self.harness.advance(Duration::from_millis(50));
    }

    fn shows(&mut self, text: &'static str) {
        settle_until(&mut self.harness, |h| h.html().contains(text));
    }

    fn lacks(&mut self, text: &'static str) {
        settle_until(&mut self.harness, |h| !h.html().contains(text));
    }
}

#[test]
fn the_window_lists_the_folder_and_leaves_hidden_files_out() {
    let rig = open();
    let html = rig.harness.html();
    for text in ["notes.txt", "Projects", "report.pdf", "zebra.png"] {
        assert!(html.contains(text), "{text} is not listed:\n{html}");
    }
    assert!(!html.contains(".secret"), "a hidden file is listed");
    // The columns beside the name: kind and size, said in words.
    for text in ["Folder", "PDF file", "4 bytes", "Date Modified"] {
        assert!(html.contains(text), "{text} is not shown:\n{html}");
    }
}

#[test]
fn the_folders_name_is_the_toolbars_title() {
    let rig = open();
    let name = rig.dir.path().file_name().unwrap().to_string_lossy().into_owned();
    let title = rig.harness.text_of(".ds-label[*|data-style=\"headline\"]");
    assert_eq!(title.as_deref(), Some(name.as_str()));
}

#[test]
fn a_click_selects_a_row() {
    let mut rig = open();
    rig.click_row(1);
    assert!(rig.selected(1), "row 1 is not selected");
    assert!(!rig.selected(0));
    rig.click_row(3);
    assert!(rig.selected(3));
    assert!(!rig.selected(1), "a plain click keeps the old row");
}

#[test]
fn typing_a_letter_jumps_to_the_name_that_starts_with_it() {
    let mut rig = open();
    rig.click_row(0);
    rig.harness.send(Input::key(ShortcutKey::Char('z')));
    rig.harness.advance(Duration::from_millis(50));
    assert!(rig.selected(3), "zebra.png is not selected");
    assert!(!rig.selected(0));
}

#[test]
fn command_down_opens_the_selected_folder_and_command_bracket_goes_back() {
    let mut rig = open();
    rig.click_row(1);
    rig.command(ShortcutKey::Down);
    rig.shows("inner.txt");
    assert!(
        rig.opener.opened().is_empty(),
        "the folder next to Projects was opened too"
    );
    rig.lacks("zebra.png");

    rig.command(ShortcutKey::Char('['));
    rig.shows("zebra.png");
}

#[test]
fn the_back_button_returns_and_command_up_goes_to_the_enclosing_folder() {
    let mut rig = open();
    rig.click_row(1);
    rig.command(ShortcutKey::Down);
    rig.shows("inner.txt");

    let back = rig
        .harness
        .centre(".ds-toolbar-leading .ds-button")
        .unwrap_or_else(|| panic!("no Back button:\n{}", rig.harness.html()));
    rig.harness.send(Input::click(back));
    rig.shows("zebra.png");

    rig.click_row(1);
    rig.command(ShortcutKey::Down);
    rig.shows("inner.txt");
    rig.command(ShortcutKey::Up);
    rig.shows("zebra.png");
}

#[test]
fn a_file_goes_to_the_opener_and_the_window_stays_where_it_is() {
    let mut rig = open();
    rig.click_row(2);
    rig.command(ShortcutKey::Char('o'));
    assert_eq!(rig.opener.opened(), vec![rig.dir.path().join("report.pdf")]);
    assert!(rig.harness.html().contains("zebra.png"));
}

#[test]
fn return_renames_in_place_and_escape_puts_the_name_back() {
    let mut rig = open();
    rig.click_row(0);
    rig.harness.send(Input::key(ShortcutKey::Enter));
    settle_until(&mut rig.harness, |h| h.count(".ds-text-field") > 0);

    rig.harness.send(Input::key(ShortcutKey::Escape));
    settle_until(&mut rig.harness, |h| h.count(".ds-text-field") == 0);
    assert!(rig.harness.html().contains("notes.txt"));
}

#[test]
fn command_shift_period_shows_hidden_files() {
    let mut rig = open();
    rig.click_row(0);
    rig.harness.send(Input::chord(
        &[ShortcutKey::Super, ShortcutKey::Shift],
        ShortcutKey::Char('.'),
    ));
    rig.shows(".secret");
}

#[test]
fn select_all_selects_every_row() {
    let mut rig = open();
    rig.click_row(0);
    rig.command(ShortcutKey::Char('a'));
    rig.harness.advance(Duration::from_millis(50));
    assert!((0..4).all(|n| rig.selected(n)), "not every row is selected");
}

#[test]
fn the_sidebar_lists_the_favorites_and_the_computer() {
    let dir = scratch_folder();
    let home = FolderPath::new(dir.path()).unwrap();
    let places = Places::default().with(PlaceKind::Home, home);
    let rig = open_with(dir, places, "zebra.png");
    let html = rig.harness.html();
    for text in ["Favorites", "Home", "Locations", "Computer"] {
        assert!(html.contains(text), "{text} is not in the sidebar:\n{html}");
    }
    assert!(!html.contains("Trash"), "the Trash row waits for the Trash");
}

#[test]
fn a_folder_that_is_gone_says_so_and_offers_to_try_again() {
    let dir = scratch_folder();
    let gone = dir.path().join("not-here");
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let mut harness = harness(launch(&gone, config.path(), &opener));
    settle_until(&mut harness, |h| {
        h.html().contains("That folder can&#39;t be found.")
            || h.html().contains("That folder can't be found.")
    });
    assert!(harness.html().contains("Retry"));
}

#[test]
fn an_empty_folder_says_so() {
    let dir = tempfile::tempdir().unwrap();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let mut harness = harness(launch(dir.path(), config.path(), &opener));
    settle_until(&mut harness, |h| h.html().contains("This folder is empty."));
}
