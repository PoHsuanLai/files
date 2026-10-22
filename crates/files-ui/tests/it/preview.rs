//! The preview pane in the window on a scratch folder (Shift+Command+P), the thumbnails of the
//! icon view, and the previews the window keeps by item.

use std::sync::Arc;
use std::time::Duration;

use anyview_core::FilePath;
use anyview_fs::OnDisk;
use anyview_image::{Rgba8, encode_png};
use anyview_peek::{Peeking, look};
use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input};
use files_core::{FileName, FolderPath, ItemRef};
use files_fs::FakePrefs;
use files_trash::FakeTrash;
use files_ui::looks::Looks;
use tempfile::TempDir;

use super::support::{RecordingOpener, at, harness, launch_remembering, scratch_folder};

/// The table row `n`. By name the scratch folder sorts `notes.txt`, `Projects`, `report.pdf`,
/// `zebra.png`.
fn row(n: usize) -> String {
    format!(".ds-vtable-item[*|data-row=\"{n}\"]")
}

/// A window on `dir`, once it has listed everything in it.
fn open(dir: &TempDir) -> (Harness, TempDir) {
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let trash = Arc::new(FakeTrash::new(config.path().join("trash")));
    let prefs = Arc::new(FakePrefs::new());
    let launch = launch_remembering(at(dir.path()), config.path(), &opener, trash, prefs);
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

/// Shift+Command+P.
fn toggle_preview(window: &mut Harness) {
    window.send(Input::chord(
        &[ShortcutKey::Super, ShortcutKey::Shift],
        ShortcutKey::Char('p'),
    ));
    window.advance(Duration::from_millis(50));
}

#[test]
fn shift_command_p_shows_and_hides_the_pane_with_the_card_and_the_details() {
    let dir = scratch_folder();
    let (mut window, _config) = open(&dir);
    click_row(&mut window, 0);
    assert_eq!(window.count(".ds-preview"), 0, "the pane starts away");

    toggle_preview(&mut window);
    settle_until(&mut window, |h| {
        h.count(".ds-preview") > 0 && h.count(".anyview-pane") > 0
    });
    let html = window.html();
    for text in [
        "notes.txt",
        "Kind",
        "TXT file",
        "Size",
        "5 bytes",
        "Modified",
    ] {
        assert!(html.contains(text), "{text} is not in the pane:\n{html}");
    }
    // The folder is still there beside it.
    assert!(window.count(".ds-vtable-item") >= 4);

    toggle_preview(&mut window);
    settle_until(&mut window, |h| h.count(".ds-preview") == 0);
    assert!(window.html().contains("zebra.png"));
}

#[test]
fn the_pane_follows_the_selection_to_none_and_to_several() {
    let dir = scratch_folder();
    let (mut window, _config) = open(&dir);
    toggle_preview(&mut window);
    settle_until(&mut window, |h| h.html().contains("No Selection"));

    // notes.txt (5), Projects (a folder), report.pdf (4) and zebra.png (3).
    window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('a')));
    settle_until(&mut window, |h| {
        h.html().contains("4 items") && h.html().contains("12 bytes")
    });

    click_row(&mut window, 1);
    // The folder holds one item, which its facts say once they are read.
    settle_until(&mut window, |h| h.html().contains("1 item"));
    assert!(!window.html().contains("4 items"));
}

#[test]
fn the_icon_view_draws_a_picture_for_an_image_and_keeps_icons_for_the_rest() {
    let dir = scratch_folder();
    let size = anyview_core::PixelSize {
        width: anyview_core::PixelLen(2),
        height: anyview_core::PixelLen(2),
    };
    let picture = Rgba8::new(size, vec![200; 16]).unwrap();
    std::fs::write(dir.path().join("pic.png"), encode_png(&picture).unwrap()).unwrap();
    let (mut window, _config) = open(&dir);
    window.send(Input::chord(&[ShortcutKey::Super], ShortcutKey::Char('1')));
    settle_until(&mut window, |h| h.count(".ds-cv-item") == 5);
    // The tile for pic.png gets its picture; the others stay icons.
    settle_until(&mut window, |h| h.count(".ds-cv-image") == 1);
    assert_eq!(window.count(".ds-cv-item"), 5);
}

#[test]
fn a_preview_is_read_only_for_the_item_it_was_made_for() {
    let dir = scratch_folder();
    let folder = FolderPath::new(dir.path()).unwrap();
    let item = |name: &str| ItemRef::new(folder.clone(), FileName::new(name).unwrap());
    let card = |name: &str| {
        let path = FilePath::new(dir.path().join(name)).unwrap();
        Arc::new(look(path.on_disk(), &Peeking::pane()))
    };
    let mut looks = Looks::default();
    assert!(looks.of(&item("notes.txt")).is_none());

    // The pane and Quick Look are both about notes.txt, then Quick Look moves on.
    looks.put(item("notes.txt"), card("notes.txt"));
    looks.put(item("report.pdf"), card("report.pdf"));
    assert_eq!(looks.of(&item("notes.txt")).unwrap().name, "notes.txt");
    assert_eq!(looks.of(&item("report.pdf")).unwrap().name, "report.pdf");
    // An answer for an item nobody asked about now is never handed to a view of another.
    assert!(looks.of(&item("zebra.png")).is_none());

    // Only the newest few are kept.
    for n in 0..6 {
        looks.put(item(&format!("f{n}")), card("notes.txt"));
    }
    assert!(looks.of(&item("notes.txt")).is_none());
    assert!(looks.of(&item("f5")).is_some());
}
