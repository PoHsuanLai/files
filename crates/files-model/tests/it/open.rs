//! Opening the selection.

use files_core::{ByteSize, Entry, EntryKind, LinkTo, Special};
use files_model::{Effect, Input, Modifier, OpenTarget, Opens, opens};

use crate::common::*;

fn kinds() -> Vec<Entry> {
    vec![
        folder("dir"),
        file("doc.txt"),
        Entry::new(name("folder-link"), EntryKind::Link(LinkTo::Folder)),
        Entry::new(
            name("file-link"),
            EntryKind::Link(LinkTo::File(ByteSize(3))),
        ),
        Entry::new(name("dead-link"), EntryKind::Link(LinkTo::Broken)),
        Entry::new(name("pipe"), EntryKind::Special(Special::Fifo)),
    ]
}

fn target(text: &str) -> OpenTarget {
    OpenTarget::new(files_core::FolderPath::new("/home/p").unwrap(), name(text))
}

#[test]
fn opening_one_entry_goes_in_or_hands_it_over_or_does_nothing() {
    // (entry clicked, if any; effects after Open; where the browser ends up)
    let table: Vec<(Option<&str>, Vec<Effect>, &str)> = vec![
        (Some("dir"), vec![Effect::Unwatch(home())], "/home/p/dir"),
        (
            Some("folder-link"),
            vec![Effect::Unwatch(home())],
            "/home/p/folder-link",
        ),
        (
            Some("doc.txt"),
            vec![Effect::OpenFiles(vec![target("doc.txt")])],
            "/home/p",
        ),
        (
            Some("file-link"),
            vec![Effect::OpenFiles(vec![target("file-link")])],
            "/home/p",
        ),
        (Some("dead-link"), vec![], "/home/p"),
        (Some("pipe"), vec![], "/home/p"),
        (None, vec![], "/home/p"),
    ];
    for (chosen, want, ends_at) in table {
        let before = chosen.map(|c| Input::Click(name(c), Modifier::None));
        let (browser, _) = run(loaded(kinds()), before.into_iter().collect());
        let (browser, effects) = browser.step(Input::Open, at(0));
        let first: Vec<_> = effects.first().cloned().into_iter().collect();
        assert_eq!(first, want, "{chosen:?}");
        assert_eq!(path_of(browser.location()), ends_at, "{chosen:?}");
        if ends_at != "/home/p" {
            assert!(effects.contains(&Effect::List(browser.location().clone())));
        }
    }
}

#[test]
fn opening_several_files_hands_over_the_files_in_shown_order() {
    let (browser, effects) = run(loaded(kinds()), vec![Input::SelectAll, Input::Open]);
    assert_eq!(
        effects,
        vec![Effect::OpenFiles(vec![
            target("doc.txt"),
            target("file-link")
        ])]
    );
    assert_eq!(path_of(browser.location()), "/home/p");
}

#[test]
fn what_opening_does_follows_the_kind() {
    let got: Vec<Opens> = kinds().iter().map(opens).collect();
    assert_eq!(
        got,
        vec![
            Opens::Folder,
            Opens::File,
            Opens::Folder,
            Opens::File,
            Opens::Nothing,
            Opens::Nothing
        ]
    );
}
