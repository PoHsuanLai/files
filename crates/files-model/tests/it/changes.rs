//! Changing files: new folder, Trash, copy and paste, drops, undo, questions and reports.

use std::time::SystemTime;

use files_core::{
    ByteSize, Entry, EntryKind, Finish, FolderPath, ItemRef, Location, OpEvent, OpFailure, OpKind,
    OpPlan, Outcome, Summary, Transfer, TrashId, TrashedItem,
};
use files_model::{Browser, Change, DropHow, Effect, Input, Modifier, Question, Rename};

use crate::common::*;

pub(super) fn home_folder() -> FolderPath {
    FolderPath::new("/home/p").unwrap()
}

fn item(text: &str) -> ItemRef {
    ItemRef::new(home_folder(), name(text))
}

fn click(text: &str) -> Input {
    Input::Click(name(text), Modifier::None)
}

fn toggle(text: &str) -> Input {
    Input::Click(name(text), Modifier::Toggle)
}

/// A browser on the Trash holding `a.txt` (from /home/p) and `b.txt` (from /home/q).
pub(super) fn trash_browser() -> Browser {
    let held = |text: &str, from: &str| {
        let original = ItemRef::new(FolderPath::new(from).unwrap(), name(text));
        let kind = EntryKind::File(ByteSize(1));
        let id = TrashId::new(format!("/t/info/{text}.trashinfo"));
        let trashed = TrashedItem::new(id, original, SystemTime::UNIX_EPOCH, kind.clone());
        Entry::new(name(text), kind).in_trash(trashed)
    };
    let (browser, _) = Browser::new(Location::Trash);
    let rows = vec![held("a.txt", "/home/p"), held("b.txt", "/home/q")];
    browser
        .step(Input::Listed(Location::Trash, Ok(rows)), at(0))
        .0
}

fn trashed(browser: &Browser, text: &str) -> TrashedItem {
    browser
        .rows()
        .iter()
        .find(|row| row.name == name(text))
        .and_then(|row| row.trashed.clone())
        .unwrap()
}

pub(super) fn summary(kind: OpKind) -> Summary {
    Summary::new(kind, 1, Some(name("x")))
}

pub(super) fn finished(kind: OpKind, finish: Finish, undo: Option<Summary>) -> Input {
    Input::Op(OpEvent::Finished(Outcome::new(summary(kind), finish, undo)))
}

#[test]
fn a_new_folder_takes_the_first_free_untitled_name() {
    let table: Vec<(&str, Vec<Entry>, &str)> = vec![
        ("empty folder", vec![], "untitled folder"),
        (
            "taken",
            vec![folder("untitled folder")],
            "untitled folder 2",
        ),
        (
            "two taken",
            vec![folder("untitled folder"), folder("untitled folder 2")],
            "untitled folder 3",
        ),
        (
            "a file by that name",
            vec![file("untitled folder")],
            "untitled folder 2",
        ),
    ];
    for (why, entries, want) in table {
        let (_, effects) = loaded(entries).step(Input::NewFolder, at(0));
        assert_eq!(
            effects,
            vec![Effect::Run(OpPlan::NewFolder {
                parent: home_folder(),
                name: name(want)
            })],
            "{why}"
        );
    }
}

#[test]
fn a_new_folder_starts_a_rename_once_it_is_listed() {
    let browser = loaded(vec![file("a.txt")]);
    let (browser, _) = browser.step(Input::NewFolder, at(0));
    assert_eq!(browser.rename(), &Rename::Idle, "not yet: it is not listed");

    // The watcher reports it, and the name opens for editing with the folder selected.
    let (browser, _) = browser.step(
        Input::Changed(home(), Change::Added(folder("untitled folder"))),
        at(0),
    );
    assert_eq!(
        browser.rename(),
        &Rename::Editing {
            name: name("untitled folder"),
            draft: "untitled folder".into(),
            problem: None
        }
    );
    assert_eq!(selected(&browser), ["untitled folder"]);

    // Only once.
    let (browser, _) = run(
        browser,
        vec![
            Input::CancelRename,
            Input::Changed(home(), Change::Updated(folder("untitled folder"))),
        ],
    );
    assert_eq!(browser.rename(), &Rename::Idle);
}

#[test]
fn a_new_folder_that_failed_does_not_start_a_rename_later() {
    let (browser, _) = loaded(vec![]).step(Input::NewFolder, at(0));
    let (browser, effects) = browser.step(
        finished(
            OpKind::NewFolder,
            Finish::Failed(OpFailure::NoPermission),
            None,
        ),
        at(0),
    );
    assert_eq!(effects, vec![Effect::Tell(OpFailure::NoPermission)]);
    let (browser, _) = browser.step(
        Input::Changed(home(), Change::Added(folder("untitled folder"))),
        at(0),
    );
    assert_eq!(browser.rename(), &Rename::Idle);
}

#[test]
fn a_new_folder_is_made_only_in_a_folder() {
    let (_, effects) = trash_browser().step(Input::NewFolder, at(0));
    assert!(effects.is_empty());
}

#[test]
fn move_to_trash_takes_the_selection_in_row_order() {
    let browser = loaded(vec![file("c"), file("a"), file("b")]);
    let (browser, _) = run(browser, vec![click("c"), toggle("a")]);
    let (_, effects) = browser.step(Input::MoveToTrash, at(0));
    assert_eq!(
        effects,
        vec![Effect::Run(OpPlan::Trash {
            items: vec![item("a"), item("c")]
        })]
    );
}

#[test]
fn move_to_trash_needs_a_selection_in_a_folder() {
    let (_, nothing) = loaded(vec![file("a")]).step(Input::MoveToTrash, at(0));
    assert!(nothing.is_empty());
    let (selected_in_trash, _) = trash_browser().step(click("a.txt"), at(0));
    let (_, none) = selected_in_trash.step(Input::MoveToTrash, at(0));
    assert!(none.is_empty());
}

#[test]
fn put_back_gives_the_trashed_items_of_the_selected_rows() {
    let browser = trash_browser();
    let (browser, _) = run(browser, vec![click("a.txt"), toggle("b.txt")]);
    let (a, b) = (trashed(&browser, "a.txt"), trashed(&browser, "b.txt"));
    let (_, effects) = browser.step(Input::PutBack, at(0));
    assert_eq!(
        effects,
        vec![Effect::Run(OpPlan::PutBack { items: vec![a, b] })]
    );
}

#[test]
fn put_back_does_nothing_in_a_folder() {
    let (browser, _) = loaded(vec![file("a")]).step(click("a"), at(0));
    let (_, effects) = browser.step(Input::PutBack, at(0));
    assert!(effects.is_empty());
}

#[test]
fn deleting_immediately_asks_first_and_then_erases_those_items() {
    let (browser, _) = trash_browser().step(click("a.txt"), at(0));
    let a = trashed(&browser, "a.txt");
    let (browser, effects) = browser.step(Input::DeleteImmediately, at(0));
    assert!(effects.is_empty());
    assert_eq!(
        browser.question(),
        &Question::DeleteForGood(vec![a.clone()])
    );

    let (browser, effects) = browser.step(Input::Confirm, at(0));
    assert_eq!(browser.question(), &Question::None);
    assert_eq!(
        effects,
        vec![Effect::Run(OpPlan::DeleteForGood { items: vec![a] })]
    );
}

#[test]
fn emptying_the_trash_asks_first_and_can_be_put_away() {
    let (asked, effects) = trash_browser().step(Input::EmptyTrash, at(0));
    assert!(effects.is_empty());
    assert_eq!(asked.question(), &Question::EmptyTrash);

    let (said_no, effects) = asked.clone().step(Input::Dismiss, at(0));
    assert_eq!((said_no.question(), effects), (&Question::None, vec![]));

    let (_, effects) = asked.step(Input::Confirm, at(0));
    assert_eq!(effects, vec![Effect::Run(OpPlan::EmptyTrash)]);
}

#[test]
fn an_empty_trash_or_a_folder_has_nothing_to_empty() {
    let (empty, _) = Browser::new(Location::Trash);
    let (empty, _) = empty.step(Input::Listed(Location::Trash, Ok(vec![])), at(0));
    for browser in [empty, loaded(vec![file("a")])] {
        let (browser, effects) = browser.step(Input::EmptyTrash, at(0));
        assert_eq!((browser.question(), effects), (&Question::None, vec![]));
    }
}

#[test]
fn copy_then_paste_copies_and_paste_move_moves_and_forgets() {
    let (browser, _) = run(
        loaded(vec![file("a"), file("b")]),
        vec![click("a"), toggle("b"), Input::Copy],
    );
    assert_eq!(browser.clipboard(), &[item("a"), item("b")]);

    // Paste in another folder.
    let elsewhere = dir("/home/q");
    let (browser, _) = browser.step(Input::Go(elsewhere.clone()), at(0));
    let transfer = Transfer::new(
        vec![item("a"), item("b")],
        FolderPath::new("/home/q").unwrap(),
    );
    let (browser, effects) = browser.step(Input::Paste, at(0));
    assert_eq!(effects, vec![Effect::Run(OpPlan::Copy(transfer.clone()))]);
    assert_eq!(browser.clipboard().len(), 2, "a paste keeps the clipboard");

    let (browser, effects) = browser.step(Input::PasteMove, at(0));
    assert_eq!(effects, vec![Effect::Run(OpPlan::Move(transfer))]);
    assert!(browser.clipboard().is_empty());
    let (_, effects) = browser.step(Input::Paste, at(0));
    assert!(effects.is_empty(), "nothing left to paste");
}

#[test]
fn copying_needs_a_selection_and_pasting_needs_a_folder() {
    let (browser, _) = loaded(vec![file("a")]).step(Input::Copy, at(0));
    assert!(browser.clipboard().is_empty());

    let (copied, _) = run(loaded(vec![file("a")]), vec![click("a"), Input::Copy]);
    let (in_trash, _) = copied.step(Input::Go(Location::Trash), at(0));
    let (_, effects) = in_trash.step(Input::Paste, at(0));
    assert!(effects.is_empty());
}

/// Why, onto, how, the sources, and the plan wanted.
type DropCase<'a> = (
    &'a str,
    Option<&'a str>,
    DropHow,
    Vec<ItemRef>,
    Option<OpPlan>,
);

#[test]
fn a_drop_moves_or_copies_to_the_folder_row_or_the_window() {
    let q = |text: &str| ItemRef::new(FolderPath::new("/home/q").unwrap(), name(text));
    let into = |folder: &str| FolderPath::new(folder).unwrap();
    let table: Vec<DropCase> = vec![
        (
            "onto the window",
            None,
            DropHow::Natural,
            vec![q("x")],
            Some(OpPlan::Drag(Transfer::new(vec![q("x")], into("/home/p")))),
        ),
        (
            "onto a folder row",
            Some("sub"),
            DropHow::Natural,
            vec![q("x")],
            Some(OpPlan::Drag(Transfer::new(
                vec![q("x")],
                into("/home/p/sub"),
            ))),
        ),
        (
            "with the copy key",
            Some("sub"),
            DropHow::Copy,
            vec![q("x")],
            Some(OpPlan::Copy(Transfer::new(
                vec![q("x")],
                into("/home/p/sub"),
            ))),
        ),
        (
            "onto a file row means the window",
            Some("a.txt"),
            DropHow::Natural,
            vec![q("x")],
            Some(OpPlan::Drag(Transfer::new(vec![q("x")], into("/home/p")))),
        ),
        (
            "back into the folder it came from",
            None,
            DropHow::Natural,
            vec![item("a.txt")],
            None,
        ),
        (
            "a copy into the folder it came from",
            None,
            DropHow::Copy,
            vec![item("a.txt")],
            Some(OpPlan::Copy(Transfer::new(
                vec![item("a.txt")],
                into("/home/p"),
            ))),
        ),
        ("nothing dropped", None, DropHow::Natural, vec![], None),
    ];
    for (why, onto, how, sources, want) in table {
        let browser = loaded(vec![file("a.txt"), folder("sub")]);
        let input = Input::Drop {
            sources,
            onto: onto.map(name),
            how,
        };
        let (_, effects) = browser.step(input, at(0));
        let want: Vec<Effect> = want.into_iter().map(Effect::Run).collect();
        assert_eq!(effects, want, "{why}");
    }
}
