//! The four coherence rules (quire's `CONSUMING.md` section 5) for files' window, as quire's
//! consumer example proves them (`examples/consumer/tests/it/coherence.rs` at the pin):
//! 1. [`our_stylesheet_lints_clean`]: no literal design value in the window's own CSS.
//! 2. [`rendered_markup_uses_only_styled_classes`] and [`no_raw_form_control_or_svg_is_rendered`]:
//!    no raw markup, no class nothing styles, in the real window with a folder listed, with a
//!    name being typed, with the context menu up (and its Open With submenu, and the sheet of
//!    every app), with the erase question up, with Quick Look up, with Get Info up and with the
//!    Go to Folder sheet up (and its words under the field), with two tabs (the tab bar), with
//!    the tag menu up (its colours checked), with a tag place listed (its rows, and the
//!    sidebar's Tags section) and with the icon view listed and with a name typed on a tile.
//! 3. Rule 3 (a missing component is added to quire first) is a review discipline, not a check;
//!    the gaps are listed in the lane's report.
//! 4. Rule 4 (motion only through quire's timers): the window draws no motion of its own.

use std::sync::Arc;
use std::time::Duration;

use ds::base::press::PointerButton;
use ds::prelude::ShortcutKey;
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use ds_lint::{LintConfig, Profile, Rule, assert_clean, markup};
use files_core::{FileName, FolderPath, ItemRef, Location, Tags};
use files_fs::{FakePrefs, FakeTagger};
use files_trash::{FakeTrash, Trash};
use files_ui::{PlaceKind, Places, STYLE};

use super::support::{RecordingOpener, harness, launch_tagging, scratch_folder};

/// What `markup` needs to know is styled: quire's own stylesheet, plus the window's.
fn window_css() -> String {
    // The card of Quick Look is anyview's and brings its own stylesheet, as it does at run time.
    format!("{}\n{STYLE}\n{}", ds::stylesheet(), anyview_peek::STYLE)
}

/// What the window is showing when its markup is taken.
#[derive(Debug, Clone, Copy)]
enum Pose {
    /// A folder listed.
    Listed,
    /// A name being typed.
    Renaming,
    /// The context menu on a row.
    Menu,
    /// The Trash with its erase question up.
    Erasing,
    /// Quick Look up over a file, its card drawn.
    QuickLook,
    /// Get Info up over a file, its facts read.
    Info,
    /// The context menu with its Open With submenu open.
    OpenWith,
    /// The sheet of every installed app, behind Open With, Other.
    OtherApps,
    /// The Go to Folder sheet, its path in the field.
    GoTo,
    /// The Go to Folder sheet after a path that is not a folder, with its words under the field.
    GoToWrong,
    /// Two tabs: the tab bar shown over the folder.
    TwoTabs,
    /// The context menu with its Tags submenu open.
    TagMenu,
    /// The Red tag place, listed.
    TagPlace,
    /// The folder as icons.
    Icons,
    /// The folder as icons with a name being typed on a tile.
    IconsRenaming,
}

const POSES: [Pose; 15] = [
    Pose::Listed,
    Pose::Renaming,
    Pose::Menu,
    Pose::Erasing,
    Pose::QuickLook,
    Pose::Info,
    Pose::OpenWith,
    Pose::OtherApps,
    Pose::GoTo,
    Pose::GoToWrong,
    Pose::TwoTabs,
    Pose::TagMenu,
    Pose::TagPlace,
    Pose::Icons,
    Pose::IconsRenaming,
];

/// The window in `pose`.
fn page(pose: Pose) -> String {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let start = match pose {
        Pose::Erasing => {
            let item = ItemRef::new(
                FolderPath::new(dir.path()).unwrap(),
                FileName::new("notes.txt").unwrap(),
            );
            trash.trash(&[item]).unwrap();
            Location::Trash
        }
        Pose::Listed
        | Pose::Renaming
        | Pose::Menu
        | Pose::QuickLook
        | Pose::Info
        | Pose::OpenWith
        | Pose::OtherApps
        | Pose::GoTo
        | Pose::GoToWrong
        | Pose::TwoTabs
        | Pose::TagMenu
        | Pose::TagPlace
        | Pose::Icons
        | Pose::IconsRenaming => super::support::at(dir.path()),
    };
    // The home folder is the scratch one, so the sidebar lists the tags; notes.txt carries two.
    let home = FolderPath::new(dir.path()).unwrap();
    let tagger = Arc::new(FakeTagger::new());
    tagger.put(
        ItemRef::new(home.clone(), FileName::new("notes.txt").unwrap()),
        Tags::parse("Red,Work"),
    );
    let launch = launch_tagging(
        start,
        config.path(),
        &opener,
        trash,
        Arc::new(FakePrefs::new()),
        tagger,
    )
    .with_home(home.clone())
    .with_places(Places::default().with(PlaceKind::Home, home));
    let mut window: Harness = harness(launch);
    settle_until(&mut window, |h| h.html().contains("notes.txt"));
    match pose {
        Pose::Listed => {}
        Pose::Renaming => {
            let at = window.centre(".ds-vtable-item[*|data-row=\"0\"]").unwrap();
            window.send(Input::click(at));
            window.advance(Duration::from_millis(50));
            window.send(Input::key(ShortcutKey::Enter));
            settle_until(&mut window, |h| h.count(".ds-text-field") > 0);
        }
        Pose::Icons | Pose::IconsRenaming => {
            window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('1')));
            settle_until(&mut window, |h| h.count(".ds-cv-item") > 0);
            if matches!(pose, Pose::IconsRenaming) {
                let at = window.centre(".ds-cv-item[*|data-index=\"0\"]").unwrap();
                window.send(Input::click(at));
                window.advance(Duration::from_millis(50));
                window.send(Input::key(ShortcutKey::Enter));
                settle_until(&mut window, |h| h.count(".ds-text-field") > 0);
            }
        }
        Pose::Menu => {
            let at = window.centre(".ds-vtable-item[*|data-row=\"0\"]").unwrap();
            window.send(Input::press(at, PointerButton::Secondary));
            settle_until(&mut window, |h| h.html().contains("Move to Trash"));
        }
        Pose::Erasing => {
            window.send(Input::chord(
                &[ShortcutKey::Super, ShortcutKey::Shift],
                ShortcutKey::Backspace,
            ));
            settle_until(&mut window, |h| h.html().contains("permanently erase"));
        }
        Pose::QuickLook => {
            let at = window.centre(".ds-vtable-item[*|data-row=\"0\"]").unwrap();
            window.send(Input::click(at));
            window.advance(Duration::from_millis(50));
            window.send(Input::key(ShortcutKey::Space));
            settle_until(&mut window, |h| {
                h.count(".ds-peek") > 0 && h.count(".anyview-pane") > 0
            });
        }
        Pose::Info => {
            let at = window.centre(".ds-vtable-item[*|data-row=\"0\"]").unwrap();
            window.send(Input::click(at));
            window.advance(Duration::from_millis(50));
            window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('i')));
            settle_until(&mut window, |h| h.html().contains("Extension"));
        }
        Pose::OpenWith | Pose::OtherApps => {
            let at = window.centre(".ds-vtable-item[*|data-row=\"2\"]").unwrap();
            window.send(Input::press(at, PointerButton::Secondary));
            settle_until(&mut window, |h| h.html().contains("Move to Trash"));
            // Down to Open With, Right into its submenu.
            for key in [ShortcutKey::Down, ShortcutKey::Down, ShortcutKey::Right] {
                window.send(Input::key(key));
                window.advance(Duration::from_millis(50));
            }
            settle_until(&mut window, |h| h.html().contains("Viewer (default)"));
            if matches!(pose, Pose::OtherApps) {
                // Past the three apps to Other, and pick it.
                for key in [
                    ShortcutKey::Down,
                    ShortcutKey::Down,
                    ShortcutKey::Down,
                    ShortcutKey::Enter,
                ] {
                    window.send(Input::key(key));
                    window.advance(Duration::from_millis(50));
                }
                settle_until(&mut window, |h| h.html().contains("Choose Application"));
            }
        }
        Pose::TwoTabs => {
            window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('t')));
            settle_until(&mut window, |h| h.html().contains("aria-label=\"Tabs\""));
        }
        Pose::TagMenu => {
            let at = window.centre(".ds-vtable-item[*|data-row=\"0\"]").unwrap();
            window.send(Input::press(at, PointerButton::Secondary));
            settle_until(&mut window, |h| h.html().contains("Move to Trash"));
            // Down to Tags, Right into its submenu.
            for _ in 0..9 {
                window.send(Input::key(ShortcutKey::Down));
                window.advance(Duration::from_millis(50));
            }
            window.send(Input::key(ShortcutKey::Right));
            settle_until(&mut window, |h| h.count(".ds-menu-item") > 12);
        }
        Pose::TagPlace => {
            let red = window
                .centre(".ds-sidebar .ds-row[*|data-leading=\"element\"]")
                .unwrap();
            window.send(Input::click(red));
            window.advance(Duration::from_millis(50));
            settle_until(&mut window, |h| !h.html().contains("zebra.png"));
        }
        Pose::GoTo | Pose::GoToWrong => {
            window.send(Input::chord(
                &[ShortcutKey::Super, ShortcutKey::Shift],
                ShortcutKey::Char('g'),
            ));
            settle_until(&mut window, |h| h.count(".ds-text-field") > 0);
            if matches!(pose, Pose::GoToWrong) {
                for key in [
                    ShortcutKey::End,
                    ShortcutKey::Char('/'),
                    ShortcutKey::Char('x'),
                    ShortcutKey::Enter,
                ] {
                    window.send(Input::key(key));
                    window.advance(Duration::from_millis(50));
                }
                settle_until(&mut window, |h| h.html().contains("be found"));
            }
        }
    }
    window.html()
}

/// Rule 1: the window's CSS under the strictest profile, the call `CONSUMING.md` section 5 shows.
#[test]
fn our_stylesheet_lints_clean() {
    assert_clean(
        STYLE,
        &LintConfig {
            profile: Profile::Strict,
            ..LintConfig::new(&ds::kits())
        },
    );
}

/// Rule 2: every class the page carries is one quire's stylesheet or the window's styles.
#[test]
fn rendered_markup_uses_only_styled_classes() {
    for pose in POSES {
        let html = page(pose);
        let offences = markup(&html, &window_css(), &LintConfig::new(&ds::kits()));
        let unstyled: Vec<_> = offences
            .iter()
            .filter(|offence| offence.rule == Rule::UnstyledClass)
            .collect();
        assert!(unstyled.is_empty(), "{unstyled:#?}\nin:\n{html}");
    }
}

/// Rule 2: no hand-written `<button>`, `<input>`, `<select>`, `<textarea>` or `<svg>`.
#[test]
fn no_raw_form_control_or_svg_is_rendered() {
    for pose in POSES {
        let html = page(pose);
        let offences = markup(&html, &window_css(), &LintConfig::new(&ds::kits()));
        let raw: Vec<_> = offences
            .iter()
            .filter(|offence| offence.rule == Rule::RawMarkup)
            .collect();
        assert!(raw.is_empty(), "{raw:#?}\nin:\n{html}");
    }
}

/// Rule 2's inline-style half: the window writes no style attribute of its own with a literal
/// colour or duration (quire's own custom properties are allowed).
#[test]
fn no_inline_style_carries_a_literal_colour_or_duration() {
    let html = page(Pose::Renaming);
    let offences = markup(&html, &window_css(), &LintConfig::new(&ds::kits()));
    let inline: Vec<_> = offences
        .iter()
        .filter(|offence| {
            matches!(
                offence.rule,
                Rule::HexColour | Rule::ColourFunction | Rule::NamedColour | Rule::RawDuration
            )
        })
        .collect();
    assert!(inline.is_empty(), "{inline:#?}\nin:\n{html}");
}
