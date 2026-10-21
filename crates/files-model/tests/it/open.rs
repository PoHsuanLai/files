//! Opening the selection.

use files_core::{ByteSize, Entry, EntryKind, LinkTo, Special};
use files_model::{Effect, Input, Modifier, OpenTarget, Opens, opens};

use crate::common::*;

fn kinds() -> Vec<Entry> {
    vec![
        folder("dir"),
        file("doc.txt"),
        Entry::new(name("folder-link"), EntryKind::Link(LinkTo::Folder)),
        Entry::new(name("file-link"), EntryKind::Link(LinkTo::File(ByteSize(3)))),
        Entry::new(name("dead-link"), EntryKind::Link(LinkTo::Broken)),
        Entry::new(name("pipe"), EntryKind::Special(Special::Fifo)),
    ]
}

fn target(text: &str) -> OpenTarget {
    OpenTarget::new(files_core::FolderPath::new("/home/p").unwrap(), name(text))
}

#[test]
fn opening_one_entry_goes_in_or_hands_it_over_or_does_nothing() {
    // (entry, effects after Open)
    let table: Vec<(&str, Vec<Effect>)> = vec![
        ("dir", vec![Effect::Unwatch(home())]),
        ("folder-link", vec![Effect::Unwatch(home())]),
        ("doc.txt", vec![Effect::OpenFiles(vec![target("doc.txt")])]),
        ("file-link", vec![Effect::OpenFiles(vec![target("file-link")])]),
        ("dead-link", vec![]),
        ("pipe", vec![]),
    ];
    for (chosen, want) in table {
        let (browser, _) = loaded(kinds()).step(Input::Click(name(chosen), Modifier::None), at(0));
        let (_, effects) = browser.step(Input::Open, at(0));
        assert_eq!(effects.first().cloned().into_iter().collect::<Vec<_>>(), want, "{chosen}");
    }
}

#[test]
fn opening_a_folder_or_a_link_to_one_enters_it() {
    for chosen in ["dir", "folder-link"] {
        let (browser, _) = loaded(kinds()).step(Input::Click(name(chosen), Modifier::None), at(0));
        let (browser, effects) = browser.step(Input::Open, at(0));
        assert_eq!(path_of(browser.location()), format!("/home/p/{chosen}"));
        assert!(effects.contains(&Effect::List(browser.location().clone())));
    }
}

#[test]
fn opening_several_files_hands_over_the_files_in_shown_order() {
    let (browser, effects) = run(
        loaded(kinds()),
        vec![
            Input::SelectAll,
            Input::Open,
        ],
    );
    assert_eq!(
        effects,
        vec![Effect::OpenFiles(vec![target("doc.txt"), target("file-link")])]
    );
    assert_eq!(path_of(browser.location()), "/home/p");
}

#[test]
fn opening_with_nothing_selected_does_nothing() {
    let (_, effects) = loaded(kinds()).step(Input::Open, at(0));
    assert!(effects.is_empty());
}

#[test]
fn an_open_target_knows_its_path() {
    assert_eq!(target("doc.txt").path(), std::path::Path::new("/home/p/doc.txt"));
}

#[test]
fn what_opening_does_follows_the_kind() {
    let got: Vec<Opens> = kinds().iter().map(opens).collect();
    assert_eq!(
        got,
        vec![Opens::Folder, Opens::File, Opens::Folder, Opens::File, Opens::Nothing, Opens::Nothing]
    );
}
