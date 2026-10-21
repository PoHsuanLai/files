//! The window in ds-harness: a real Blitz document with the app's root over a scratch folder,
//! driven by the events a window would deliver. The disk is a scratch folder's, the look is read
//! from a scratch config folder, and nothing opens a real program.

use std::sync::Arc;
use std::time::Duration;

use ds::base::press::PointerButton;
use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_core::{FileName, FolderPath, ItemRef};
use files_trash::{FakeTrash, Trash};
use files_ui::{PlaceKind, Places};
use tempfile::TempDir;

use super::support::{RecordingOpener, harness, launch, launch_with, scratch_folder};

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

    fn chord(&mut self, held: &[ShortcutKey], key: ShortcutKey) {
        self.harness.send(Input::chord(held, key));
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
    let name = rig
        .dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
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
fn the_sidebar_lists_the_favorites_the_computer_and_the_trash() {
    let dir = scratch_folder();
    let home = FolderPath::new(dir.path()).unwrap();
    let places = Places::default().with(PlaceKind::Home, home);
    let rig = open_with(dir, places, "zebra.png");
    let html = rig.harness.html();
    for text in ["Favorites", "Home", "Locations", "Computer", "Trash"] {
        assert!(html.contains(text), "{text} is not in the sidebar:\n{html}");
    }
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

/// The first and the second button of an alert, in the markup's order.
const FIRST_BUTTON: &str = ".ds-alert-slot:nth-child(1) > .ds-button";

fn press_at(harness: &mut Harness, selector: &str) {
    let at: Point = harness
        .centre(selector)
        .unwrap_or_else(|| panic!("no {selector}:\n{}", harness.html()));
    harness.send(Input::click(at));
    harness.advance(Duration::from_millis(50));
}

#[test]
fn moving_to_the_trash_leaves_the_folder_and_the_toast_undoes_it() {
    let mut rig = open();
    let notes = rig.dir.path().join("notes.txt");
    rig.click_row(0);
    rig.chord(&[ShortcutKey::Super], ShortcutKey::Backspace);
    settle_until(&mut rig.harness, |_| !notes.exists());

    // The toast says what happened, with an Undo.
    rig.shows("to the Trash");
    assert!(
        rig.harness.html().contains("Moved"),
        "{}",
        rig.harness.html()
    );
    assert_eq!(rig.harness.count(".ds-toast-action"), 1);
    press_at(&mut rig.harness, ".ds-toast-action");
    settle_until(&mut rig.harness, |_| notes.exists());
    assert_eq!(std::fs::read_to_string(&notes).unwrap(), "hello");
}

#[test]
fn undo_from_the_keyboard_takes_back_the_last_change() {
    let mut rig = open();
    let notes = rig.dir.path().join("notes.txt");
    rig.click_row(0);
    rig.chord(&[ShortcutKey::Super], ShortcutKey::Backspace);
    settle_until(&mut rig.harness, |_| !notes.exists());
    rig.command(ShortcutKey::Char('z'));
    settle_until(&mut rig.harness, |_| notes.exists());
}

#[test]
fn a_new_folder_appears_with_its_name_ready_to_be_typed() {
    let mut rig = open();
    rig.chord(
        &[ShortcutKey::Super, ShortcutKey::Shift],
        ShortcutKey::Char('n'),
    );
    settle_until(&mut rig.harness, |h| {
        h.html().contains("untitled folder") && h.count(".ds-text-field") > 0
    });
    assert!(rig.dir.path().join("untitled folder").is_dir());
}

#[test]
fn a_taken_name_asks_and_keep_both_names_the_new_one_with_a_number() {
    let dir = scratch_folder();
    std::fs::write(dir.path().join("Projects").join("report.pdf"), "old").unwrap();
    let mut rig = open_with(dir, Places::default(), "zebra.png");
    // Copy report.pdf, go into Projects, paste.
    rig.click_row(2);
    rig.command(ShortcutKey::Char('c'));
    rig.click_row(1);
    rig.chord(&[ShortcutKey::Super], ShortcutKey::Down);
    rig.shows("inner.txt");
    rig.command(ShortcutKey::Char('v'));

    rig.shows("already exists in this location");
    assert!(rig.harness.html().contains("Do you want to replace it"));
    press_at(&mut rig.harness, FIRST_BUTTON);
    let kept = rig.dir.path().join("Projects").join("report 2.pdf");
    settle_until(&mut rig.harness, |_| kept.exists());
    assert_eq!(std::fs::read_to_string(&kept).unwrap(), "%PDF");
    assert_eq!(
        std::fs::read_to_string(rig.dir.path().join("Projects").join("report.pdf")).unwrap(),
        "old"
    );
    rig.lacks("already exists in this location");
}

#[test]
fn emptying_the_trash_asks_first_and_cancel_keeps_everything() {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let item = ItemRef::new(
        FolderPath::new(dir.path()).unwrap(),
        FileName::new("notes.txt").unwrap(),
    );
    trash.trash(&[item]).unwrap();
    let launch = launch_with(
        files_core::Location::Trash,
        config.path(),
        &opener,
        trash.clone(),
    );
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains("notes.txt"));
    assert!(harness.html().contains("Where From"));

    let empty = || {
        Input::chord(
            &[ShortcutKey::Super, ShortcutKey::Shift],
            ShortcutKey::Backspace,
        )
    };
    harness.send(empty());
    settle_until(&mut harness, |h| {
        h.html()
            .contains("permanently erase the items in the Trash")
    });
    assert!(harness.html().contains("You can"));
    harness.send(Input::key(ShortcutKey::Escape));
    settle_until(&mut harness, |h| !h.html().contains("permanently erase"));
    assert_eq!(trash.list().unwrap().len(), 1, "cancel erased nothing");

    harness.send(empty());
    settle_until(&mut harness, |h| h.html().contains("permanently erase"));
    press_at(&mut harness, FIRST_BUTTON);
    settle_until(&mut harness, |h| h.html().contains("Trash is empty"));
    assert!(trash.list().unwrap().is_empty());
}

#[test]
fn a_right_click_on_a_row_selects_it_and_offers_what_can_be_done() {
    let mut rig = open();
    let row = rig.harness.centre(&row(2)).unwrap();
    rig.harness
        .send(Input::press(row, PointerButton::Secondary));
    rig.harness.advance(Duration::from_millis(300));
    assert!(rig.selected(2), "the row under the pointer is selected");
    let html = rig.harness.html();
    for text in ["Open", "Move to Trash", "Rename", "Copy", "New Folder"] {
        assert!(html.contains(text), "{text} is not in the menu:\n{html}");
    }
    assert!(!html.contains("Put Back"), "that is the Trash's");
}

#[test]
fn an_empty_trash_says_so() {
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let launch = launch_with(files_core::Location::Trash, config.path(), &opener, trash);
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains("Trash is empty"));
}
