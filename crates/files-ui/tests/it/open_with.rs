//! Open With in the window: the submenu of the row's context menu, and the sheet behind "Other…".
//! The menu is driven by keys, as a person on a keyboard would: Down to the line, Right into the
//! submenu, Return to pick.

use std::sync::Arc;
use std::time::Duration;

use ds::base::press::PointerButton;
use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_model::AppPick;
use files_ui::OpenAs;
use tempfile::TempDir;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

/// A window on the scratch folder with what it needs kept alive.
struct Rig {
    harness: Harness,
    opener: Arc<RecordingOpener>,
    dir: TempDir,
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
        opener,
        dir,
        _config: config,
    }
}

impl Rig {
    /// The context menu on `report.pdf`, the third row.
    fn menu_on_report(&mut self) {
        let at: Point = self
            .harness
            .centre(".ds-vtable-item[*|data-row=\"2\"]")
            .unwrap();
        self.harness
            .send(Input::press(at, PointerButton::Secondary));
        settle_until(&mut self.harness, |h| h.html().contains("Move to Trash"));
    }

    fn key(&mut self, key: ShortcutKey) {
        self.harness.send(Input::key(key));
        self.harness.advance(Duration::from_millis(50));
    }

    /// Down to "Open With" (the second line), and Right into its submenu.
    fn into_submenu(&mut self) {
        self.menu_on_report();
        self.key(ShortcutKey::Down);
        self.key(ShortcutKey::Down);
        self.key(ShortcutKey::Right);
        settle_until(&mut self.harness, |h| h.html().contains("Viewer (default)"));
    }
}

fn viewer() -> AppPick {
    AppPick::new("viewer.desktop")
}

#[test]
fn the_submenu_lists_the_default_first_then_the_rest_by_name_then_other() {
    let mut rig = open();
    rig.into_submenu();
    let html = rig.harness.html();
    let at = |text: &str| {
        html.find(text)
            .unwrap_or_else(|| panic!("no {text}:\n{html}"))
    };
    let order = [
        at("Viewer (default)"),
        at(">editor<"),
        at(">Zed<"),
        at("Other\u{2026}"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
    assert!(!html.contains("Calculator"), "that is for the Other sheet");
}

#[test]
fn picking_the_default_opens_the_file_with_it() {
    let mut rig = open();
    rig.into_submenu();
    rig.key(ShortcutKey::Enter);
    settle_until(&mut rig.harness, |_| !rig.opener.calls().is_empty());
    assert_eq!(
        rig.opener.calls(),
        vec![(
            vec![rig.dir.path().join("report.pdf")],
            OpenAs::App(viewer())
        )]
    );
}

#[test]
fn other_lists_every_app_and_a_pick_opens_with_it() {
    let mut rig = open();
    rig.into_submenu();
    // Viewer, editor, Zed, then Other.
    for _ in 0..3 {
        rig.key(ShortcutKey::Down);
    }
    rig.key(ShortcutKey::Enter);
    settle_until(&mut rig.harness, |h| {
        h.html().contains("Choose Application")
    });
    let html = rig.harness.html();
    for text in ["Calculator", "Viewer", "Zed"] {
        assert!(html.contains(text), "{text} is not listed:\n{html}");
    }
    assert!(!html.contains("(default)"), "the sheet marks no default");
    assert!(rig.opener.calls().is_empty(), "nothing opens before a pick");

    let first: Point = rig.harness.centre(".ds-sheet-body .ds-row").unwrap();
    rig.harness.send(Input::click(first));
    settle_until(&mut rig.harness, |_| !rig.opener.calls().is_empty());
    assert_eq!(
        rig.opener.calls(),
        vec![(
            vec![rig.dir.path().join("report.pdf")],
            OpenAs::App(AppPick::new("calc.desktop"))
        )]
    );
    settle_until(&mut rig.harness, |h| {
        !h.html().contains("Choose Application")
    });
}

#[test]
fn escape_puts_the_sheet_of_apps_away_and_opens_nothing() {
    let mut rig = open();
    rig.into_submenu();
    for _ in 0..3 {
        rig.key(ShortcutKey::Down);
    }
    rig.key(ShortcutKey::Enter);
    settle_until(&mut rig.harness, |h| {
        h.html().contains("Choose Application")
    });
    rig.key(ShortcutKey::Escape);
    settle_until(&mut rig.harness, |h| {
        !h.html().contains("Choose Application")
    });
    assert!(rig.opener.calls().is_empty());
}

#[test]
fn the_row_menu_now_offers_quick_look_get_info_and_copy_as_pathname() {
    let mut rig = open();
    rig.menu_on_report();
    let html = rig.harness.html();
    for text in [
        "Open With",
        "Get Info",
        "Quick Look",
        "Copy as Pathname",
        "New Folder",
    ] {
        assert!(html.contains(text), "{text} is not in the menu:\n{html}");
    }
    // Only the menu holds these words, so the order of the page is the order of the menu.
    let menu = &html[html.find("Open With").unwrap()..];
    let at = |text: &str| {
        menu.find(text)
            .unwrap_or_else(|| panic!("no {text}:\n{menu}"))
    };
    let order = [
        at("Move to Trash"),
        at("Get Info"),
        at("Rename"),
        at("Quick Look"),
        at("Copy as Pathname"),
        at("New Folder"),
    ];
    assert!(order.windows(2).all(|pair| pair[0] < pair[1]), "{order:?}");
}
