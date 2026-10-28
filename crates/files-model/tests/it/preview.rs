//! The preview pane: whether it is up, what it asks for as the selection changes, and an answer
//! for an item it is no longer about.

use files_core::{ByteSize, EntryKind, FolderPath, Info, ItemRef, Location};
use files_model::{Browser, Effect, Input, ListFailure, Modifier, Preview, Tabbed, Tabs};

use crate::common::*;

fn item(text: &str) -> ItemRef {
    ItemRef::new(FolderPath::new("/home/p").unwrap(), name(text))
}

fn click(text: &str) -> Input {
    Input::Click(name(text), Modifier::None)
}

fn info_of(text: &str) -> Info {
    Info::new(item(text), EntryKind::File(ByteSize(1)))
}

fn asks(text: &str) -> Vec<Effect> {
    vec![Effect::Look(item(text)), Effect::ReadDetails(item(text))]
}

fn three() -> Browser {
    loaded(vec![file("a"), file("b"), folder("d")])
}

#[test]
fn the_pane_is_away_until_asked_for_and_toggles() {
    let (browser, effects) = run(three(), vec![click("a")]);
    assert!(!browser.pane_shown());
    assert_eq!(browser.preview(), Preview::Hidden);
    assert!(effects.is_empty(), "nothing is asked for while it is away");

    let (browser, effects) = run(browser, vec![Input::TogglePreview]);
    assert!(browser.pane_shown());
    // Up over a selected file, it asks for that file at once.
    assert_eq!(effects, asks("a"));

    let (browser, effects) = run(browser, vec![Input::TogglePreview]);
    assert_eq!(browser.preview(), Preview::Hidden);
    assert!(effects.is_empty());
    assert_eq!(browser.details(), None);
}

#[test]
fn the_pane_asks_for_the_one_selected_item_and_counts_several() {
    // label, inputs after the pane is up, what it is about, what it asked for
    let cases: Vec<(&str, Vec<Input>, Preview, Vec<Effect>)> = vec![
        ("nothing selected", vec![], Preview::Nothing, vec![]),
        (
            "one file",
            vec![click("a")],
            Preview::One(item("a")),
            asks("a"),
        ),
        (
            "a folder is an item too",
            vec![click("d")],
            Preview::One(item("d")),
            asks("d"),
        ),
        (
            "the next item replaces it",
            vec![click("a"), click("b")],
            Preview::One(item("b")),
            [asks("a"), asks("b")].concat(),
        ),
        (
            "the same item again asks for nothing more",
            vec![click("a"), click("a")],
            Preview::One(item("a")),
            asks("a"),
        ),
        (
            "two files weigh together",
            vec![click("a"), Input::Click(name("b"), Modifier::Toggle)],
            Preview::Many {
                count: 2,
                size: ByteSize(2),
            },
            asks("a"),
        ),
        (
            "a folder weighs nothing",
            vec![click("a"), Input::Click(name("d"), Modifier::Toggle)],
            Preview::Many {
                count: 2,
                size: ByteSize(1),
            },
            asks("a"),
        ),
        (
            "select none empties it",
            vec![click("a"), Input::SelectNone],
            Preview::Nothing,
            asks("a"),
        ),
    ];
    for (label, inputs, preview, asked) in cases {
        let (browser, _) = run(three(), vec![Input::TogglePreview]);
        let (browser, effects) = run(browser, inputs);
        assert_eq!(browser.preview(), preview, "{label}");
        assert_eq!(effects, asked, "{label}");
    }
}

#[test]
fn facts_for_another_item_or_a_failed_read_are_ignored() {
    let (browser, _) = run(three(), vec![Input::TogglePreview, click("a")]);
    // The selection moved on to b before a's facts came back.
    let (browser, _) = run(browser, vec![click("b")]);
    let (stale, _) = browser
        .clone()
        .step(Input::DetailsRead(item("a"), Ok(info_of("a"))), at(0));
    assert_eq!(stale.details(), None);

    let (failed, _) = browser.clone().step(
        Input::DetailsRead(item("b"), Err(ListFailure::NotFound)),
        at(0),
    );
    assert_eq!(failed.details(), None);

    let (read, effects) = browser.step(Input::DetailsRead(item("b"), Ok(info_of("b"))), at(0));
    assert!(effects.is_empty());
    assert_eq!(read.details(), Some(&info_of("b")));
    // Moving on drops the facts of the item left.
    let (moved, _) = run(read, vec![click("a")]);
    assert_eq!(moved.details(), None);
}

#[test]
fn the_pane_is_nothing_in_the_trash() {
    let (browser, _) = Browser::new(Location::Trash);
    let (browser, _) = browser.step(Input::Listed(Location::Trash, Ok(vec![file("a")])), at(0));
    let (browser, effects) = run(browser, vec![Input::TogglePreview, click("a")]);
    assert_eq!(browser.preview(), Preview::Nothing);
    assert!(effects.is_empty());
}

#[test]
fn the_tabs_share_the_pane_and_a_tab_brought_forward_asks_for_its_item() {
    let (tabs, _) = Tabs::new(home());
    let first = tabs.front_id();
    let (tabs, _) = tabs.step(Input::Listed(home(), Ok(vec![file("a"), file("b")])), at(0));
    let (tabs, _) = tabs.step(click("a"), at(0));
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    assert!(!tabs.get(first).unwrap().browser().pane_shown());

    // Put up in the front tab, it is up in the other too, and only the front tab asked.
    let (tabs, effects) = tabs.step(Input::TogglePreview, at(0));
    assert!(tabs.get(first).unwrap().browser().pane_shown());
    assert!(effects.is_empty(), "the new tab has nothing selected yet");

    // The first tab has a selection and was behind: coming forward asks for its item.
    let (tabs, effects) = tabs.step(Input::ShowTab(first), at(0));
    let want = Tabbed::all(first, asks("a"));
    assert_eq!(effects, want);
    assert_eq!(tabs.front_id(), first);
}
