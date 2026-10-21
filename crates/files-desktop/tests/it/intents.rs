//! Files offered to the agent layer: a call from "the router" (the connection the test names as
//! intentd, on a bus of its own) reaches a fake window, and nobody else is answered.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use docket_core::{
    ActivatedInvocation, AppRefusal, Args, CallId, FileRef, Invocation, Manifest, Origin, Outcome,
    ParamName, TargetValue, UndoFault, Undoable, Value, validate,
};
use files_core::{FileName, FolderPath, ItemRef, Location, OpKind};
use files_desktop::intents::{self, MANIFEST};
use files_desktop::{Ask, Refusal, Reply, Undoing, Window};
use porter_core::AppName;
use prov::{ActionName, Actor, Integrity, Label, Labelled, SpaceId};
use zbus::zvariant::Value as BusValue;

use crate::bus::PrivateBus;

const PROVIDER_PATH: &str = "/org/quire/IntentProvider1";
const PROVIDER_INTERFACE: &str = "org.quire.IntentProvider1";

/// A window that remembers what it was asked and answers from a fixed state.
#[derive(Default)]
struct FakeWindow {
    asked: Mutex<Vec<Ask>>,
    selected: Vec<PathBuf>,
    place: Option<Location>,
}

impl FakeWindow {
    fn asked(&self) -> Vec<Ask> {
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .clone()
    }
}

impl Window for FakeWindow {
    fn ask(&self, ask: Ask) -> impl Future<Output = Result<Reply, Refusal>> + Send {
        let reply = match &ask {
            Ask::Selection => Reply::Items(self.selected.clone()),
            Ask::CurrentFolder => match &self.place {
                Some(place) => Reply::Place(place.clone()),
                None => Reply::Done,
            },
            Ask::Open(_)
            | Ask::Trash(_)
            | Ask::NewFolder { .. }
            | Ask::Copy { .. }
            | Ask::Move { .. }
            | Ask::EmptyTrash
            | Ask::Undo(_)
            | _ => Reply::Done,
        };
        self.asked
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .push(ask);
        async move { Ok(reply) }
    }
}

fn param(name: &str) -> ParamName {
    ParamName::parse(name).unwrap()
}

fn invocation(action: &str, target: TargetValue, args: &[(&str, Value)]) -> String {
    let args: Args = args
        .iter()
        .map(|(name, value)| {
            (
                param(name),
                Labelled::new(value.clone(), Label::trusted_user()),
            )
        })
        .collect();
    let call = Invocation {
        call: CallId(1),
        action: ActionName::parse(action).unwrap(),
        target,
        args,
        actor: Actor::User {
            via: AppName::parse("org.quire.Shell").unwrap(),
        },
        origin: Origin::Launcher,
        space: SpaceId::parse("work").unwrap(),
    };
    serde_json::to_string(&ActivatedInvocation {
        invocation: call,
        activation: None,
        classified: None,
    })
    .unwrap()
}

fn files(paths: &[&str]) -> TargetValue {
    TargetValue::Files(paths.iter().map(|p| FileRef::parse(p).unwrap()).collect())
}

fn file(path: &str) -> Value {
    Value::File(FileRef::parse(path).unwrap())
}

struct Served {
    _bus: PrivateBus,
    window: Arc<FakeWindow>,
    router: zbus::Connection,
    _server: zbus::Connection,
}

async fn served(window: FakeWindow) -> Option<Served> {
    let bus = PrivateBus::start("intents")?;
    let window = Arc::new(window);
    let server = bus.connect().await;
    let router = bus.connect().await;
    intents::offer_for_router(&server, window.clone(), &router)
        .await
        .unwrap();
    Some(Served {
        _bus: bus,
        window,
        router,
        _server: server,
    })
}

async fn call(
    caller: &zbus::Connection,
    member: &str,
    arguments: &(impl serde::Serialize + zbus::zvariant::DynamicType),
) -> zbus::Result<String> {
    let reply = caller
        .call_method(
            Some(intents::APP_BUS_NAME),
            PROVIDER_PATH,
            Some(PROVIDER_INTERFACE),
            member,
            arguments,
        )
        .await?;
    reply.body().deserialize()
}

async fn perform(
    caller: &zbus::Connection,
    action: &str,
    target: TargetValue,
    args: &[(&str, Value)],
) -> Result<Outcome, AppRefusal> {
    let options: HashMap<String, BusValue<'_>> = HashMap::new();
    let text = call(
        caller,
        "Perform",
        &(invocation(action, target, args), options),
    )
    .await
    .unwrap();
    serde_json::from_str(&text).unwrap()
}

fn path(text: &str) -> PathBuf {
    PathBuf::from(text)
}

fn item(folder: &str, name: &str) -> ItemRef {
    ItemRef::new(
        FolderPath::new(folder).unwrap(),
        FileName::new(name).unwrap(),
    )
}

#[test]
fn the_shipped_file_is_one_the_router_accepts_and_every_action_is_named() {
    let parsed: Manifest = toml::from_str(MANIFEST).unwrap();
    let valid = validate(parsed).unwrap();
    let mut names: Vec<&str> = valid
        .manifest()
        .actions
        .iter()
        .map(|a| a.name.as_str())
        .collect();
    names.sort_unstable();
    assert_eq!(
        names,
        [
            intents::COPY,
            intents::CURRENT_FOLDER,
            intents::EMPTY_TRASH,
            intents::MOVE,
            intents::NEW_FOLDER,
            intents::OPEN,
            intents::SELECTION,
            intents::TRASH,
        ]
    );
}

#[test]
fn an_undo_token_names_the_change_it_takes_back() {
    let cases = [
        (OpKind::Trash, 3, "undo:trash:3"),
        (OpKind::Copy, 1, "undo:copy:1"),
        (OpKind::Move, 12, "undo:move:12"),
        (OpKind::NewFolder, 1, "undo:new_folder:1"),
    ];
    for (kind, count, text) in cases {
        let undoing = Undoing { kind, count };
        assert_eq!(undoing.token().as_deref(), Some(text));
        assert_eq!(Undoing::parse(text), Some(undoing));
    }
    for bad in [
        "",
        "undo:",
        "undo:trash",
        "undo:trash:x",
        "undo:rename:1",
        "pane:1",
    ] {
        assert_eq!(Undoing::parse(bad), None, "{bad}");
    }
}

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
    let intruder = s._bus.connect().await;
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
