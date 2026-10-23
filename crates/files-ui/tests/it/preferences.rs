//! The person's choices in the window: which sidebar rows are listed, and folders on top.

use std::sync::Arc;

use ds_harness::harness::settle_until;
use files_core::{Behaviour, FolderPath, FoldersOnTop};
use files_ui::{DiskRows, PlaceKind, Places, Preferences, Sidebar};
use tokio::sync::watch;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

#[test]
fn a_row_the_person_left_out_is_not_in_the_sidebar() {
    let dir = scratch_folder();
    let folder = FolderPath::new(dir.path()).unwrap();
    let places = Places::default()
        .with(PlaceKind::Music, folder.clone())
        .with(PlaceKind::Pictures, folder);
    let preferences = Preferences {
        sidebar: Sidebar::default().leaving_out(PlaceKind::Pictures),
        ..Preferences::default()
    };
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let launch = launch(dir.path(), config.path(), &opener)
        .with_places(places)
        .with_preferences(watch::channel(preferences).1);
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains("zebra.png"));
    let html = harness.html();
    assert!(
        html.contains("Music"),
        "Music is not in the sidebar:\n{html}"
    );
    assert!(!html.contains("Pictures"), "Pictures is in the sidebar");
}

#[test]
fn the_rows_the_sidebar_lists_follow_the_choice() {
    let places = Places::default()
        .with(PlaceKind::Home, FolderPath::new("/h").unwrap())
        .with(PlaceKind::Music, FolderPath::new("/m").unwrap());
    let sidebar = Sidebar::default()
        .leaving_out(PlaceKind::Home)
        .leaving_out(PlaceKind::Computer)
        .leaving_out(PlaceKind::Trash)
        .with_disks(DiskRows::Left);
    let listed = |places: &Places| {
        files_ui::places::groups(places)
            .into_iter()
            .map(|group| group.rows)
            .collect::<Vec<_>>()
    };
    assert_eq!(
        listed(&places.clone().with_sidebar(sidebar.clone())),
        vec![vec![PlaceKind::Music], vec![PlaceKind::Trash]],
        "the Trash cannot be left out"
    );
    assert_eq!(sidebar.disks(), DiskRows::Left);
    assert_eq!(Sidebar::default().disks(), DiskRows::Listed);
}

#[test]
fn folders_come_first_when_the_person_asks() {
    let table = [
        ("mixed", FoldersOnTop::Mixed, "notes.txt", "Projects"),
        ("first", FoldersOnTop::First, "Projects", "notes.txt"),
    ];
    for (label, folders, earlier, later) in table {
        let dir = scratch_folder();
        let preferences = Preferences {
            behaviour: Behaviour {
                folders,
                ..Behaviour::default()
            },
            ..Preferences::default()
        };
        let config = tempfile::tempdir().unwrap();
        let opener = Arc::new(RecordingOpener::default());
        let launch = launch(dir.path(), config.path(), &opener)
            .with_preferences(watch::channel(preferences).1);
        let mut harness = harness(launch);
        settle_until(&mut harness, |h| h.html().contains("zebra.png"));
        let html = harness.html();
        let (a, b) = (html.find(earlier), html.find(later));
        assert!(
            a < b && a.is_some(),
            "{label}: {earlier} is not above {later}"
        );
    }
}
