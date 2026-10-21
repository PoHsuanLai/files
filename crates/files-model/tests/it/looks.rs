//! Quick Look and Get Info: what opens, what follows the cursor, what puts them away.

use files_core::{ByteSize, EntryKind, FileName, FolderPath, Info, ItemRef, Location};
use files_model::{
    Browser, Change, Direction, Effect, InfoView, Input, ListFailure, Looking, Modifier,
};

use crate::common::*;

fn item(text: &str) -> ItemRef {
    ItemRef::new(FolderPath::new("/home/p").unwrap(), name(text))
}

fn info_of(text: &str) -> Info {
    Info::new(item(text), EntryKind::File(ByteSize(1500)))
}

fn two_files() -> Browser {
    loaded(vec![file("a"), file("b")])
}

fn click(text: &str) -> Input {
    Input::Click(name(text), Modifier::None)
}

fn shown(browser: &Browser) -> Option<String> {
    browser
        .quick_look()
        .map(|entry| entry.name.display().into_owned())
}

#[test]
fn space_opens_quick_look_on_the_cursor_row_and_asks_for_its_preview() {
    let (browser, effects) = run(two_files(), vec![click("a"), Input::QuickLook]);
    assert_eq!(shown(&browser), Some("a".to_owned()));
    assert_eq!(effects, vec![Effect::Look(item("a"))]);
}

#[test]
fn quick_look_toggles_and_has_nothing_to_show_without_a_row() {
    // label, inputs, what is shown after them
    let cases: Vec<(&str, Vec<Input>, Option<&str>)> = vec![
        ("nothing selected", vec![Input::QuickLook], None),
        ("opened", vec![click("a"), Input::QuickLook], Some("a")),
        (
            "toggled shut",
            vec![click("a"), Input::QuickLook, Input::QuickLook],
            None,
        ),
        (
            "closed by Escape",
            vec![click("a"), Input::QuickLook, Input::CloseQuickLook],
            None,
        ),
        (
            "closing what is not up changes nothing",
            vec![click("a"), Input::CloseQuickLook],
            None,
        ),
    ];
    for (label, inputs, want) in cases {
        let (browser, _) = run(two_files(), inputs);
        assert_eq!(shown(&browser).as_deref(), want, "{label}");
    }
}

#[test]
fn the_preview_follows_the_arrow_keys_and_is_asked_for_once_per_item() {
    let (browser, _) = run(two_files(), vec![click("a"), Input::QuickLook]);
    let down = Input::MoveCursor(Direction::Down, Modifier::None);
    let (browser, effects) = run(browser, vec![down.clone()]);
    assert_eq!(shown(&browser), Some("b".to_owned()));
    assert_eq!(effects, vec![Effect::Look(item("b"))]);
    // At the last row the cursor stays, and nothing new is asked for.
    let (browser, effects) = run(browser, vec![down]);
    assert_eq!(shown(&browser), Some("b".to_owned()));
    assert!(effects.is_empty());
    assert_eq!(browser.quick_item(), Some(item("b")));
}

#[test]
fn quick_look_goes_away_with_its_row_or_its_folder() {
    let (browser, _) = run(two_files(), vec![click("a"), Input::QuickLook]);
    let (removed, _) = browser
        .clone()
        .step(Input::Changed(home(), Change::Removed(name("a"))), at(0));
    assert_eq!(removed.quick_look(), None);
    let (left, _) = browser.step(Input::Go(dir("/x")), at(0));
    assert_eq!(left.quick_look(), None);
}

#[test]
fn quick_look_does_not_open_in_the_trash() {
    let (browser, _) = Browser::new(Location::Trash);
    let (browser, _) = browser.step(Input::Listed(Location::Trash, Ok(vec![file("a")])), at(0));
    let (browser, effects) = run(browser, vec![click("a"), Input::QuickLook]);
    assert_eq!(browser.quick_look(), None);
    assert!(effects.is_empty());
}

#[test]
fn get_info_is_about_the_cursor_row_or_else_the_folder() {
    let (browser, effects) = run(two_files(), vec![click("b"), Input::GetInfo]);
    assert_eq!(effects, vec![Effect::ReadInfo(item("b"))]);
    assert_eq!(browser.info(), &InfoView::Loading(item("b")));

    let (browser, effects) = run(two_files(), vec![Input::GetInfo]);
    let folder = ItemRef::new(
        FolderPath::new("/home").unwrap(),
        FileName::new("p").unwrap(),
    );
    assert_eq!(effects, vec![Effect::ReadInfo(folder.clone())]);
    assert_eq!(browser.info(), &InfoView::Loading(folder));
}

#[test]
fn get_info_shows_what_was_read_and_ignores_a_stale_answer() {
    let (browser, _) = run(two_files(), vec![click("a"), Input::GetInfo]);
    let (stale, _) = browser
        .clone()
        .step(Input::InfoRead(item("b"), Ok(info_of("b"))), at(0));
    assert_eq!(stale.info(), &InfoView::Loading(item("a")));
    let (browser, effects) = browser.step(Input::InfoRead(item("a"), Ok(info_of("a"))), at(0));
    assert!(effects.is_empty());
    assert_eq!(browser.info(), &InfoView::Shown(info_of("a")));
    let (browser, _) = browser.step(Input::CloseInfo, at(0));
    assert_eq!(browser.info(), &InfoView::Closed);
}

#[test]
fn get_info_that_cannot_be_read_says_so() {
    let (browser, _) = run(
        two_files(),
        vec![
            click("a"),
            Input::GetInfo,
            Input::InfoRead(item("a"), Err(ListFailure::NotFound)),
        ],
    );
    assert_eq!(
        browser.info(),
        &InfoView::Failed(item("a"), ListFailure::NotFound)
    );
}

#[test]
fn get_info_and_quick_look_do_not_stack() {
    let (browser, _) = run(
        two_files(),
        vec![click("a"), Input::QuickLook, Input::GetInfo],
    );
    assert_eq!(browser.quick_look(), None);
    assert!(matches!(browser.info(), InfoView::Loading(_)));
    let (browser, _) = browser.step(Input::QuickLook, at(0));
    assert_eq!(browser.quick_look(), None);
    assert_eq!(Looking::default(), Looking::Closed);
}

#[test]
fn get_info_does_nothing_in_the_trash() {
    let (browser, _) = Browser::new(Location::Trash);
    let (browser, effects) = browser.step(Input::GetInfo, at(0));
    assert_eq!(browser.info(), &InfoView::Closed);
    assert!(!effects.contains(&Effect::ReadInfo(item("a"))));
    assert!(effects.is_empty());
}

#[test]
fn properties_reveal_the_item_and_open_get_info_on_it() {
    let (browser, _) = Browser::new(home());
    let folder = FolderPath::new("/srv").unwrap();
    let (browser, effects) = browser.step(
        Input::Properties {
            folder: folder.clone(),
            names: vec![name("x"), name("y")],
        },
        at(0),
    );
    let asked = ItemRef::new(folder.clone(), name("x"));
    assert!(effects.contains(&Effect::List(dir("/srv"))));
    assert!(effects.contains(&Effect::ReadInfo(asked.clone())));
    assert_eq!(browser.info(), &InfoView::Loading(asked));

    // With no name, it is the folder's own.
    let (browser, _) = Browser::new(home());
    let (_, effects) = browser.step(
        Input::Properties {
            folder,
            names: Vec::new(),
        },
        at(0),
    );
    let own = ItemRef::new(FolderPath::new("/").unwrap(), name("srv"));
    assert!(effects.contains(&Effect::ReadInfo(own)));
}
