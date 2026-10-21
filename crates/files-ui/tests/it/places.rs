//! The sidebar's places: what is listed, where a row goes, which row the cursor is on.

use files_core::{FolderPath, Location, TagColour};
use files_ui::places::{Group, groups, title_words};
use files_ui::{Heading, PlaceKey, PlaceKind, Places};

fn folder(path: &str) -> FolderPath {
    FolderPath::new(path).unwrap()
}

fn place(path: &str) -> Location {
    Location::Folder(folder(path))
}

fn person() -> Places {
    Places::default()
        .with(PlaceKind::Home, folder("/home/p"))
        .with(PlaceKind::Desktop, folder("/home/p/Desktop"))
        .with(PlaceKind::Downloads, folder("/home/p/Downloads"))
        .with(PlaceKind::Music, folder("/home/p/Music"))
}

#[test]
fn the_sidebar_lists_the_favorites_that_have_a_folder_then_the_computer_and_the_trash() {
    assert_eq!(
        groups(&person()),
        vec![
            Group {
                heading: Heading::Favorites,
                rows: vec![
                    PlaceKind::Home,
                    PlaceKind::Desktop,
                    PlaceKind::Downloads,
                    PlaceKind::Music,
                ],
            },
            Group {
                heading: Heading::Locations,
                rows: vec![PlaceKind::Computer, PlaceKind::Trash],
            },
            Group {
                heading: Heading::Tags,
                rows: TagColour::ALL.map(PlaceKind::Tag).to_vec(),
            },
        ]
    );
}

#[test]
fn a_tag_row_goes_to_its_colours_search_and_the_cursor_finds_it_there() {
    let places = person();
    let red = Location::Tag(TagColour::Red);
    assert_eq!(
        places.location_of(PlaceKind::Tag(TagColour::Red)),
        Some(red.clone())
    );
    assert_eq!(
        places.key_of(&red),
        Some(PlaceKey::Place(PlaceKind::Tag(TagColour::Red)))
    );
    // A search starts from the home folder: with none the colours are not listed.
    let homeless = Places::default().with(PlaceKind::Desktop, folder("/d"));
    assert_eq!(homeless.location_of(PlaceKind::Tag(TagColour::Red)), None);
    assert_eq!(title_words(&red), "Red");
}

#[test]
fn a_person_with_no_favorites_still_has_the_computer_and_the_trash() {
    assert_eq!(
        groups(&Places::default()),
        vec![Group {
            heading: Heading::Locations,
            rows: vec![PlaceKind::Computer, PlaceKind::Trash],
        }]
    );
}

#[test]
fn the_order_is_the_sidebars_not_the_order_they_were_given_in() {
    let places = Places::default()
        .with(PlaceKind::Videos, folder("/v"))
        .with(PlaceKind::Home, folder("/h"));
    let favorites = &groups(&places)[0].rows;
    assert_eq!(favorites, &[PlaceKind::Home, PlaceKind::Videos]);
}

#[test]
fn a_row_goes_to_its_folder_and_the_computer_to_the_root() {
    let places = person();
    assert_eq!(
        places.location_of(PlaceKind::Downloads),
        Some(place("/home/p/Downloads"))
    );
    assert_eq!(places.location_of(PlaceKind::Pictures), None);
    assert_eq!(places.location_of(PlaceKind::Computer), Some(place("/")));
    assert_eq!(places.location_of(PlaceKind::Trash), Some(Location::Trash));
}

#[test]
fn a_later_folder_replaces_an_earlier_one() {
    let places = Places::default()
        .with(PlaceKind::Home, folder("/a"))
        .with(PlaceKind::Home, folder("/b"));
    assert_eq!(places.location_of(PlaceKind::Home), Some(place("/b")));
}

#[test]
fn the_cursor_is_on_the_place_the_window_is_in() {
    let places = person();
    let cases = [
        (
            "home",
            place("/home/p"),
            Some(PlaceKey::Place(PlaceKind::Home)),
        ),
        (
            "a favorite",
            place("/home/p/Music"),
            Some(PlaceKey::Place(PlaceKind::Music)),
        ),
        (
            "the root",
            place("/"),
            Some(PlaceKey::Place(PlaceKind::Computer)),
        ),
        (
            "a folder inside a place is none",
            place("/home/p/Music/Album"),
            None,
        ),
        (
            "the trash",
            Location::Trash,
            Some(PlaceKey::Place(PlaceKind::Trash)),
        ),
    ];
    for (name, location, want) in cases {
        assert_eq!(places.key_of(&location), want, "{name}");
    }
}

#[test]
fn two_places_in_one_folder_show_the_first() {
    let places = Places::default()
        .with(PlaceKind::Desktop, folder("/home/p"))
        .with(PlaceKind::Home, folder("/home/p"));
    assert_eq!(
        places.key_of(&place("/home/p")),
        Some(PlaceKey::Place(PlaceKind::Home))
    );
}

#[test]
fn the_title_is_the_folders_own_name() {
    let cases = [
        ("a folder", place("/home/p/Documents"), "Documents"),
        ("home", place("/home/p"), "p"),
        ("the root", place("/"), "/"),
        ("the trash", Location::Trash, "Trash"),
    ];
    for (name, location, want) in cases {
        assert_eq!(title_words(&location), want, "{name}");
    }
}
