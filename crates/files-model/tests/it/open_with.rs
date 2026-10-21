//! Open With: the selection goes to the app picked, and the sheet of every app.

use files_core::{Entry, EntryKind, FolderPath, Special};
use files_model::{AppPick, Chooser, Effect, Input, Modifier, OpenTarget};

use crate::common::*;

fn pick() -> AppPick {
    AppPick::new("viewer.desktop")
}

fn target(text: &str) -> OpenTarget {
    OpenTarget::new(FolderPath::new("/home/p").unwrap(), name(text))
}

fn entries() -> Vec<Entry> {
    vec![
        folder("dir"),
        file("a.txt"),
        file("b.txt"),
        Entry::new(name("pipe"), EntryKind::Special(Special::Fifo)),
    ]
}

fn click(text: &str) -> Input {
    Input::Click(name(text), Modifier::None)
}

#[test]
fn open_with_hands_the_selection_to_the_app_picked() {
    // label, inputs before, effects after
    let cases: Vec<(&str, Vec<Input>, Vec<Effect>)> = vec![
        (
            "one file",
            vec![click("a.txt")],
            vec![Effect::OpenFilesWith {
                targets: vec![target("a.txt")],
                app: pick(),
            }],
        ),
        (
            "a folder goes to the app too",
            vec![click("dir")],
            vec![Effect::OpenFilesWith {
                targets: vec![target("dir")],
                app: pick(),
            }],
        ),
        (
            "several, in shown order",
            vec![Input::SelectAll],
            vec![Effect::OpenFilesWith {
                targets: vec![target("a.txt"), target("b.txt"), target("dir")],
                app: pick(),
            }],
        ),
        ("something that opens nothing", vec![click("pipe")], vec![]),
        ("nothing selected", vec![], vec![]),
    ];
    for (label, before, want) in cases {
        let (browser, _) = run(loaded(entries()), before);
        let (after, effects) = browser.step(Input::OpenWith(pick()), at(0));
        assert_eq!(effects, want, "{label}");
        assert_eq!(path_of(after.location()), "/home/p", "{label}: stays put");
    }
}

#[test]
fn the_sheet_of_apps_opens_for_a_selection_and_closes_with_a_pick() {
    let (browser, _) = run(loaded(entries()), vec![click("a.txt"), Input::ChooseApp]);
    assert_eq!(browser.chooser(), Chooser::Open);

    let (closed, effects) = browser.clone().step(Input::CloseChooseApp, at(0));
    assert_eq!(closed.chooser(), Chooser::Closed);
    assert!(effects.is_empty());

    let (picked, effects) = browser.step(Input::OpenWith(pick()), at(0));
    assert_eq!(picked.chooser(), Chooser::Closed);
    assert_eq!(effects.len(), 1);

    let (none, _) = loaded(entries()).step(Input::ChooseApp, at(0));
    assert_eq!(none.chooser(), Chooser::Closed, "nothing to open");
}

#[test]
fn going_elsewhere_puts_the_sheet_away() {
    let (browser, _) = run(loaded(entries()), vec![click("a.txt"), Input::ChooseApp]);
    let (browser, _) = browser.step(Input::Go(dir("/tmp")), at(0));
    assert_eq!(browser.chooser(), Chooser::Closed);
}

#[test]
fn an_app_pick_is_its_desktop_id() {
    assert_eq!(pick().as_str(), "viewer.desktop");
}
