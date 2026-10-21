//! Changes another program asked for run as the same plans the person's own would, and the
//! question before emptying the Trash comes up wherever the window is.

use files_core::{ByteSize, Entry, EntryKind, FolderPath, ItemRef, Location, OpPlan, Transfer};
use files_model::{Bring, Effect, Input, Question};

use crate::common::*;

fn folder_path(text: &str) -> FolderPath {
    FolderPath::new(text).unwrap()
}

fn item(folder: &str, text: &str) -> ItemRef {
    ItemRef::new(folder_path(folder), name(text))
}

fn run(plan: OpPlan) -> Vec<Effect> {
    vec![Effect::Run(plan)]
}

fn make(parent: &str, text: &str) -> OpPlan {
    OpPlan::NewFolder {
        parent: folder_path(parent),
        name: name(text),
    }
}

#[test]
fn items_named_by_where_they_are_become_ordinary_plans() {
    let sources = vec![item("/a", "x"), item("/b", "y")];
    let into = folder_path("/dest");
    let cases: Vec<(&str, Input, Vec<Effect>)> = vec![
        (
            "trash",
            Input::TrashItems(sources.clone()),
            run(OpPlan::Trash {
                items: sources.clone(),
            }),
        ),
        ("trash nothing", Input::TrashItems(Vec::new()), vec![]),
        (
            "copy",
            Input::Bring {
                sources: sources.clone(),
                destination: into.clone(),
                how: Bring::Copy,
            },
            run(OpPlan::Copy(Transfer::new(sources.clone(), into.clone()))),
        ),
        (
            "move",
            Input::Bring {
                sources: sources.clone(),
                destination: into.clone(),
                how: Bring::Move,
            },
            run(OpPlan::Move(Transfer::new(sources.clone(), into.clone()))),
        ),
        (
            "bring nothing",
            Input::Bring {
                sources: Vec::new(),
                destination: into,
                how: Bring::Move,
            },
            vec![],
        ),
    ];
    for (label, input, want) in cases {
        let (_, effects) = loaded(Vec::new()).step(input, at(0));
        assert_eq!(effects, want, "{label}");
    }
}

#[test]
fn a_new_folder_is_named_as_asked_or_numbered_past_what_the_shown_folder_holds() {
    let held = vec![
        Entry::new(name("untitled folder"), EntryKind::Folder),
        Entry::new(name("a.txt"), EntryKind::File(ByteSize(1))),
    ];
    let make_in = |parent: &str, given: Option<&str>| Input::MakeFolder {
        parent: folder_path(parent),
        name: given.map(name),
    };
    let cases: Vec<(&str, Input, Vec<Effect>)> = vec![
        (
            "named",
            make_in("/home/p", Some("Work")),
            run(make("/home/p", "Work")),
        ),
        (
            "unnamed, in the shown folder, past a taken name",
            make_in("/home/p", None),
            run(make("/home/p", "untitled folder 2")),
        ),
        (
            "unnamed, elsewhere",
            make_in("/elsewhere", None),
            run(make("/elsewhere", "untitled folder")),
        ),
    ];
    for (label, input, want) in cases {
        let (browser, effects) = loaded(held.clone()).step(input, at(0));
        assert_eq!(effects, want, "{label}");
        assert!(
            browser.cursor().is_none(),
            "{label}: the new folder is not put into renaming"
        );
    }
}

#[test]
fn emptying_the_trash_is_asked_first_from_any_place() {
    let (browser, effects) = loaded(Vec::new()).step(Input::AskEmptyTrash, at(0));
    assert_eq!(browser.question(), &Question::EmptyTrash);
    assert_eq!(effects, vec![]);
    assert_eq!(
        browser.location(),
        &Location::Folder(folder_path("/home/p"))
    );

    let (_, effects) = browser.step(Input::Confirm, at(0));
    assert_eq!(effects, run(OpPlan::EmptyTrash));
}
