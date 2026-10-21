//! Calls from "the router" into the app's intents, answered by a fake window: what each action asks of
//! the window, what comes back, and the undo tokens.

use std::collections::HashMap;

use docket_core::{AppRefusal, Outcome, TargetValue, UndoFault, Undoable, Value};
use files_core::{FileName, FolderPath, Location, OpKind};
use files_desktop::intents;
use files_desktop::{Ask, Undoing};
use porter_core::AppName;
use prov::{Actor, Integrity, Labelled};
use zbus::zvariant::Value as BusValue;

use super::intents::{FakeWindow, call, file, files, invocation, item, path, perform, served};

#[tokio::test]
async fn open_shows_a_path_in_the_window() {
    let Some(s) = served(FakeWindow::default()).await else {
        return;
    };
    let done = perform(
        &s.router,
        intents::OPEN,
        TargetValue::Nothing,
        &[("path", file("/home/p/Documents"))],
    )
    .await
    .unwrap();
    assert_eq!(done.undo, Undoable::No);
    assert_eq!(s.window.asked(), vec![Ask::Open(path("/home/p/Documents"))]);

    let relative = perform(
        &s.router,
        intents::OPEN,
        TargetValue::Nothing,
        &[("path", file("Documents"))],
    )
    .await;
    assert!(
        matches!(relative, Err(AppRefusal::Failed(_))),
        "{relative:?}"
    );
    let missing = perform(&s.router, intents::OPEN, TargetValue::Nothing, &[]).await;
    assert!(
        matches!(missing, Err(AppRefusal::NeedsParam { .. })),
        "{missing:?}"
    );
    assert_eq!(s.window.asked().len(), 1, "a refused call asks nothing");
}

#[tokio::test]
async fn the_selection_and_the_folder_come_back_as_untrusted_file_names() {
    let window = FakeWindow {
        selected: vec![path("/home/p/a.txt"), path("/home/p/b b.txt")],
        place: Some(Location::Folder(FolderPath::new("/home/p").unwrap())),
        ..FakeWindow::default()
    };
    let Some(s) = served(window).await else {
        return;
    };
    let text_of = |outcome: &Outcome| match outcome.value.as_ref() {
        Some(Labelled {
            value: Value::Text(text),
            label,
        }) => (text.clone(), label.integrity),
        other => panic!("not text: {other:?}"),
    };
    let selected = perform(&s.router, intents::SELECTION, TargetValue::Nothing, &[])
        .await
        .unwrap();
    assert_eq!(
        text_of(&selected),
        (
            "/home/p/a.txt\n/home/p/b b.txt".to_owned(),
            Integrity::Untrusted
        )
    );
    let folder = perform(
        &s.router,
        intents::CURRENT_FOLDER,
        TargetValue::Nothing,
        &[],
    )
    .await
    .unwrap();
    assert_eq!(
        text_of(&folder),
        ("/home/p".to_owned(), Integrity::Untrusted)
    );
}

#[tokio::test]
async fn changes_reach_the_window_as_typed_asks_and_come_with_a_token_that_undoes_them() {
    let Some(s) = served(FakeWindow::default()).await else {
        return;
    };
    let trashed = perform(
        &s.router,
        intents::TRASH,
        files(&["/home/p/a.txt", "/home/p/b.txt"]),
        &[],
    )
    .await
    .unwrap();
    let Undoable::Yes(token) = trashed.undo else {
        panic!("a move to the Trash is undoable");
    };
    assert_eq!(token.as_str(), "undo:trash:2");

    perform(
        &s.router,
        intents::NEW_FOLDER,
        TargetValue::Nothing,
        &[
            ("folder", file("/home/p")),
            ("name", Value::Text("Work".to_owned())),
        ],
    )
    .await
    .unwrap();
    perform(
        &s.router,
        intents::NEW_FOLDER,
        TargetValue::Nothing,
        &[("folder", file("/home/p"))],
    )
    .await
    .unwrap();
    perform(
        &s.router,
        intents::COPY,
        files(&["/a/x"]),
        &[("to", file("/b"))],
    )
    .await
    .unwrap();
    perform(
        &s.router,
        intents::MOVE,
        files(&["/a/x", "/a/y"]),
        &[("to", file("/b"))],
    )
    .await
    .unwrap();
    perform(&s.router, intents::EMPTY_TRASH, TargetValue::Nothing, &[])
        .await
        .unwrap();

    let parent = FolderPath::new("/home/p").unwrap();
    let to = FolderPath::new("/b").unwrap();
    assert_eq!(
        s.window.asked(),
        vec![
            Ask::Trash(vec![item("/home/p", "a.txt"), item("/home/p", "b.txt")]),
            Ask::NewFolder {
                parent: parent.clone(),
                name: Some(FileName::new("Work").unwrap()),
            },
            Ask::NewFolder { parent, name: None },
            Ask::Copy {
                sources: vec![item("/a", "x")],
                to: to.clone(),
            },
            Ask::Move {
                sources: vec![item("/a", "x"), item("/a", "y")],
                to,
            },
            Ask::EmptyTrash,
        ]
    );

    let actor = serde_json::to_string(&Actor::User {
        via: AppName::parse("org.quire.Shell").unwrap(),
    })
    .unwrap();
    let quoted = format!("\"{}\"", token.as_str());
    let text = call(&s.router, "Undo", &(quoted, actor)).await.unwrap();
    let undone: Result<(), UndoFault> = serde_json::from_str(&text).unwrap();
    assert_eq!(undone, Ok(()));
    assert_eq!(
        s.window.asked().last(),
        Some(&Ask::Undo(Undoing {
            kind: OpKind::Trash,
            count: 2
        }))
    );
}

#[tokio::test]
async fn nobody_but_the_router_is_answered() {
    let Some(s) = served(FakeWindow::default()).await else {
        return;
    };
    let intruder = s.bus.connect().await;
    let options: HashMap<String, BusValue<'_>> = HashMap::new();
    let refused = call(
        &intruder,
        "Perform",
        &(
            invocation(intents::EMPTY_TRASH, TargetValue::Nothing, &[]),
            options,
        ),
    )
    .await;
    assert!(refused.is_err());
    assert!(s.window.asked().is_empty());
}
