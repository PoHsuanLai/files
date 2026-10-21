//! Tabs in the window: ⌘T opens one, the tab bar shows from two on, each tab keeps its folder.

use std::sync::Arc;
use std::time::Duration;

use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use tempfile::TempDir;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

/// The tab bar's label, as it is in the markup.
const BAR: &str = "[aria-label=\"Tabs\"]";

/// A window on the scratch folder, with what it needs kept alive.
struct Rig {
    harness: Harness,
    _dir: TempDir,
    _config: TempDir,
}

fn open() -> Rig {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let mut harness = harness(launch(dir.path(), config.path(), &opener));
    settle_until(&mut harness, |h| h.html().contains("zebra.png"));
    Rig {
        harness,
        _dir: dir,
        _config: config,
    }
}

impl Rig {
    fn click_row(&mut self, n: usize) {
        let selector = format!(".ds-vtable-item[*|data-row=\"{n}\"]");
        let at: Point = self.harness.centre(&selector).unwrap();
        self.harness.send(Input::click(at));
        self.harness.advance(Duration::from_millis(50));
    }

    fn command(&mut self, key: ShortcutKey) {
        self.harness.send(Input::chord(&[ShortcutKey::Super], key));
        self.harness.advance(Duration::from_millis(50));
    }

    fn command_shift(&mut self, key: ShortcutKey) {
        let held = [ShortcutKey::Super, ShortcutKey::Shift];
        self.harness.send(Input::chord(&held, key));
        self.harness.advance(Duration::from_millis(50));
    }

    fn shows(&mut self, text: &'static str) {
        settle_until(&mut self.harness, |h| h.html().contains(text));
    }

    fn lacks(&mut self, text: &'static str) {
        settle_until(&mut self.harness, |h| !h.html().contains(text));
    }

    fn bar(&self) -> bool {
        self.harness.centre(BAR).is_some()
    }
}

#[test]
fn command_t_opens_a_tab_on_the_same_folder_and_the_bar_shows_from_two_tabs() {
    let mut rig = open();
    assert!(!rig.bar(), "one tab shows no bar");
    rig.command(ShortcutKey::Char('t'));
    settle_until(&mut rig.harness, |h| h.centre(BAR).is_some());
    rig.shows("zebra.png");
    // Both tabs are titled with the folder's name.
    let name = rig
        ._dir
        .path()
        .file_name()
        .unwrap()
        .to_string_lossy()
        .into_owned();
    let html = rig.harness.html();
    assert!(
        html.matches(&format!(">{name}<")).count() >= 2,
        "two tabs are not titled {name}:\n{html}"
    );
}

#[test]
fn closing_the_second_tab_with_command_w_takes_the_bar_away_and_keeps_the_folder() {
    let mut rig = open();
    rig.command(ShortcutKey::Char('t'));
    settle_until(&mut rig.harness, |h| h.centre(BAR).is_some());
    rig.command(ShortcutKey::Char('w'));
    settle_until(&mut rig.harness, |h| h.centre(BAR).is_none());
    rig.shows("zebra.png");
}

#[test]
fn switching_tabs_keeps_each_tabs_folder() {
    let mut rig = open();
    // Tab one goes into Projects; tab two opens on it and goes up again.
    rig.click_row(1);
    rig.command(ShortcutKey::Down);
    rig.shows("inner.txt");
    rig.command(ShortcutKey::Char('t'));
    settle_until(&mut rig.harness, |h| h.centre(BAR).is_some());
    rig.shows("inner.txt");
    rig.command(ShortcutKey::Up);
    rig.shows("zebra.png");
    rig.lacks("inner.txt");

    // Shift+Command+[ goes to tab one, which is still in Projects.
    rig.command_shift(ShortcutKey::Char('['));
    rig.shows("inner.txt");
    rig.lacks("zebra.png");

    // Shift+Command+] comes back to tab two, still in the folder above.
    rig.command_shift(ShortcutKey::Char(']'));
    rig.shows("zebra.png");
    rig.lacks("inner.txt");
}

#[test]
fn a_tab_opened_from_the_menu_on_a_folder_opens_it_and_leaves_the_first_tab_alone() {
    let mut rig = open();
    rig.click_row(1);
    let at = rig
        .harness
        .centre(".ds-vtable-item[*|data-row=\"1\"]")
        .unwrap();
    rig.harness
        .send(Input::press(at, ds::base::press::PointerButton::Secondary));
    rig.shows("Open in New Tab");
    // Down to Open, Down to Open in New Tab, Return.
    for key in [ShortcutKey::Down, ShortcutKey::Down, ShortcutKey::Enter] {
        rig.harness.send(Input::key(key));
        rig.harness.advance(Duration::from_millis(50));
    }
    settle_until(&mut rig.harness, |h| h.centre(BAR).is_some());
    rig.shows("inner.txt");
    rig.command_shift(ShortcutKey::Char('['));
    rig.shows("zebra.png");
}

#[test]
fn a_file_row_has_no_open_in_new_tab() {
    let mut rig = open();
    rig.click_row(0);
    let at = rig
        .harness
        .centre(".ds-vtable-item[*|data-row=\"0\"]")
        .unwrap();
    rig.harness
        .send(Input::press(at, ds::base::press::PointerButton::Secondary));
    rig.shows("Move to Trash");
    assert!(!rig.harness.html().contains("Open in New Tab"));
}
