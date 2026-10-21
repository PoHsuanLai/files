//! The model's rows as the table wants them: cursor, selection, sort and clicks.

use std::time::Duration;

use ds::base::selection::PickHow;
use ds::components::lists::table::model::SortDirection;
use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, Location, SortKey, SortOrder};
use files_model::{Browser, Direction, Input, Modifier, Now};
use files_ui::rows::{Column, cursor_of, direction_of, modifier_of, order_of, selection_of};

fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

/// A browser on `/home/p` that has read `a`, `b`, `c`, `d` (sorted by name).
fn browser() -> Browser {
    let at = Location::Folder(FolderPath::new("/home/p").unwrap());
    let (browser, _) = Browser::new(at.clone());
    let entries = ["d", "b", "a", "c"]
        .into_iter()
        .map(|n| Entry::new(name(n), EntryKind::File(ByteSize(1))))
        .collect();
    let (browser, _) = browser.step(Input::Listed(at, Ok(entries)), Now(Duration::ZERO));
    browser
}

fn step(browser: Browser, input: Input) -> Browser {
    browser.step(input, Now(Duration::ZERO)).0
}

fn picked(browser: &Browser) -> Vec<usize> {
    selection_of(browser).rows().collect()
}

#[test]
fn nothing_is_selected_before_a_click() {
    let browser = browser();
    assert_eq!(cursor_of(&browser), None);
    assert!(selection_of(&browser).is_empty());
}

#[test]
fn a_click_selects_the_row_by_its_index_in_the_sorted_rows() {
    let browser = step(browser(), Input::Click(name("c"), Modifier::None));
    assert_eq!(cursor_of(&browser), Some(2));
    assert_eq!(picked(&browser), vec![2]);
    assert_eq!(selection_of(&browser).anchor(), Some(2));
}

#[test]
fn shift_extends_and_command_toggles() {
    let browser = step(browser(), Input::Click(name("a"), Modifier::None));
    let extended = step(browser.clone(), Input::Click(name("c"), Modifier::Extend));
    assert_eq!(picked(&extended), vec![0, 1, 2]);
    assert_eq!(cursor_of(&extended), Some(2));
    assert_eq!(selection_of(&extended).anchor(), Some(0));

    let toggled = step(browser, Input::Click(name("d"), Modifier::Toggle));
    assert_eq!(picked(&toggled), vec![0, 3]);
}

#[test]
fn select_all_picks_every_row() {
    let browser = step(browser(), Input::SelectAll);
    assert_eq!(picked(&browser), vec![0, 1, 2, 3]);
}

#[test]
fn the_cursor_follows_the_arrows() {
    let browser = step(
        browser(),
        Input::MoveCursor(Direction::Down, Modifier::None),
    );
    assert_eq!(cursor_of(&browser), Some(0));
    let browser = step(browser, Input::MoveCursor(Direction::End, Modifier::None));
    assert_eq!(cursor_of(&browser), Some(3));
}

#[test]
fn a_reach_means_the_modifier_it_was_made_with() {
    assert_eq!(modifier_of(PickHow::Replace), Modifier::None);
    assert_eq!(modifier_of(PickHow::Toggle), Modifier::Toggle);
    assert_eq!(modifier_of(PickHow::Extend), Modifier::Extend);
}

#[test]
fn columns_and_sort_keys_round_trip() {
    for column in Column::ALL {
        assert_eq!(Column::of_key(column.key()), column);
    }
    let keys = [
        SortKey::Name,
        SortKey::Modified,
        SortKey::Size,
        SortKey::Kind,
    ];
    for key in keys {
        assert_eq!(Column::of_key(key).key(), key);
    }
}

#[test]
fn the_directions_round_trip() {
    for order in [SortOrder::Ascending, SortOrder::Descending] {
        assert_eq!(order_of(direction_of(order)), order);
    }
    assert_eq!(
        direction_of(SortOrder::Descending),
        SortDirection::Descending
    );
}

#[test]
fn the_headers_say_what_finder_says() {
    let titles: Vec<&str> = Column::ALL.iter().map(|c| c.title()).collect();
    assert_eq!(titles, ["Name", "Date Modified", "Size", "Kind"]);
}
