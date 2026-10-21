//! Files offered to the agent layer: a call from "the router" (the connection the test names as
//! intentd, on a bus of its own) reaches a fake window, and nobody else is answered.

use std::collections::HashMap;
use std::future::Future;
use std::path::PathBuf;
use std::sync::{Arc, Mutex, PoisonError};

use docket_core::{
    ActivatedInvocation, AppRefusal, Args, CallId, FileRef, Invocation, Manifest, Origin, Outcome,
    ParamName, TargetValue, Value, validate,
};
use files_core::{FileName, FolderPath, ItemRef, Location, OpKind};
use files_desktop::intents::{self, MANIFEST};
use files_desktop::{Ask, Refusal, Reply, Undoing, Window};
use porter_core::AppName;
use prov::{ActionName, Actor, Label, Labelled, SpaceId};
use zbus::zvariant::Value as BusValue;

use crate::bus::PrivateBus;

const PROVIDER_PATH: &str = "/org/quire/IntentProvider1";
const PROVIDER_INTERFACE: &str = "org.quire.IntentProvider1";

/// A window that remembers what it was asked and answers from a fixed state.
#[derive(Default)]
pub(super) struct FakeWindow {
    pub(super) asked: Mutex<Vec<Ask>>,
    pub(super) selected: Vec<PathBuf>,
    pub(super) place: Option<Location>,
}

impl FakeWindow {
    pub(super) fn asked(&self) -> Vec<Ask> {
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

pub(super) fn invocation(action: &str, target: TargetValue, args: &[(&str, Value)]) -> String {
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

pub(super) fn files(paths: &[&str]) -> TargetValue {
    TargetValue::Files(paths.iter().map(|p| FileRef::parse(p).unwrap()).collect())
}

pub(super) fn file(path: &str) -> Value {
    Value::File(FileRef::parse(path).unwrap())
}

pub(super) struct Served {
    _bus: PrivateBus,
    pub(super) window: Arc<FakeWindow>,
    pub(super) router: zbus::Connection,
    _server: zbus::Connection,
}

pub(super) async fn served(window: FakeWindow) -> Option<Served> {
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

pub(super) async fn call(
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

pub(super) async fn perform(
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

pub(super) fn path(text: &str) -> PathBuf {
    PathBuf::from(text)
}

pub(super) fn item(folder: &str, name: &str) -> ItemRef {
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
