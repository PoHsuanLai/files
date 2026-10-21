//! Renaming in place.

use files_core::{FolderPath, NameError};
use files_model::{Browser, Change, Effect, Input, Modifier, Rename, RenameProblem};

use crate::common::*;

fn editing(draft: &str) -> Browser {
    let browser = loaded(vec![file("a.txt"), file("b.txt")]);
    let (browser, _) = run(
        browser,
        vec![Input::Click(name("a.txt"), Modifier::None), Input::BeginRename, Input::EditDraft(draft.into())],
    );
    browser
}

fn home_path() -> FolderPath {
    FolderPath::new("/home/p").unwrap()
}

#[test]
fn beginning_a_rename_opens_the_field_on_the_selected_name() {
    let browser = loaded(vec![file("a.txt"), file("b.txt")]);
    let (browser, _) = run(
        browser,
        vec![Input::Click(name("a.txt"), Modifier::None), Input::BeginRename],
    );
    assert_eq!(
        browser.rename(),
        &Rename::Editing { name: name("a.txt"), draft: "a.txt".into(), problem: None }
    );
}

#[test]
fn beginning_a_rename_needs_exactly_one_selected_entry() {
    let two = vec![
        Input::Click(name("a.txt"), Modifier::None),
        Input::Click(name("b.txt"), Modifier::Toggle),
        Input::BeginRename,
    ];
    for inputs in [vec![Input::BeginRename], two] {
        let (browser, effects) = run(loaded(vec![file("a.txt"), file("b.txt")]), inputs);
        assert_eq!(browser.rename(), &Rename::Idle);
        assert!(effects.is_empty());
    }
}

#[test]
fn committing_a_draft_does_what_the_draft_deserves() {
    let rename = |to: &str| Effect::Rename {
        folder: home_path(),
        from: name("a.txt"),
        to: name(to),
    };
    // (draft, effects, rename state afterwards)
    let table: Vec<(&str, Vec<Effect>, Rename)> = vec![
        ("c.txt", vec![rename("c.txt")], Rename::Idle),
        ("a.txt", vec![], Rename::Idle),
        (
            "b.txt",
            vec![],
            Rename::Editing {
                name: name("a.txt"),
                draft: "b.txt".into(),
                problem: Some(RenameProblem::Taken),
            },
        ),
        (
            "",
            vec![],
            Rename::Editing {
                name: name("a.txt"),
                draft: String::new(),
                problem: Some(RenameProblem::Invalid(NameError::Empty)),
            },
        ),
        (
            "x/y",
            vec![],
            Rename::Editing {
                name: name("a.txt"),
                draft: "x/y".into(),
                problem: Some(RenameProblem::Invalid(NameError::Slash)),
            },
        ),
    ];
    for (draft, want_effects, want_state) in table {
        let (browser, effects) = editing(draft).step(Input::CommitRename, at(0));
        assert_eq!(effects, want_effects, "{draft:?}");
        assert_eq!(browser.rename(), &want_state, "{draft:?}");
    }
}

#[test]
fn a_problem_is_worded_for_a_person() {
    assert_eq!(RenameProblem::Taken.to_string(), "That name is taken.");
    assert_eq!(
        RenameProblem::Invalid(NameError::Empty).to_string(),
        "A name can't be empty."
    );
}

#[test]
fn typing_in_the_field_clears_the_complaint() {
    let (browser, _) = editing("b.txt").step(Input::CommitRename, at(0));
    let (browser, _) = browser.step(Input::EditDraft("c.txt".into()), at(0));
    assert_eq!(
        browser.rename(),
        &Rename::Editing { name: name("a.txt"), draft: "c.txt".into(), problem: None }
    );
}

#[test]
fn cancelling_closes_the_field_and_asks_for_nothing() {
    let (browser, effects) = editing("zzz").step(Input::CancelRename, at(0));
    assert_eq!(browser.rename(), &Rename::Idle);
    assert!(effects.is_empty());
}

#[test]
fn committing_with_nothing_open_does_nothing() {
    let browser = loaded(vec![file("a.txt")]);
    let (after, effects) = browser.clone().step(Input::CommitRename, at(0));
    assert_eq!(after, browser);
    assert!(effects.is_empty());
}

#[test]
fn the_selection_is_frozen_while_a_name_is_edited() {
    let browser = editing("a.txt");
    let (after, _) = browser.clone().step(Input::Click(name("b.txt"), Modifier::None), at(0));
    assert_eq!(after, browser);
}

#[test]
fn the_field_closes_if_its_entry_disappears() {
    let change = Input::Changed(home(), Change::Removed(name("a.txt")));
    let (browser, _) = editing("x").step(change, at(0));
    assert_eq!(browser.rename(), &Rename::Idle);
}

#[test]
fn a_name_taken_by_a_hidden_entry_is_still_taken() {
    let browser = loaded(vec![file("a.txt"), file(".b")]);
    let (browser, _) = run(
        browser,
        vec![
            Input::Click(name("a.txt"), Modifier::None),
            Input::BeginRename,
            Input::EditDraft(".b".into()),
            Input::CommitRename,
        ],
    );
    assert!(matches!(
        browser.rename(),
        Rename::Editing { problem: Some(RenameProblem::Taken), .. }
    ));
}
