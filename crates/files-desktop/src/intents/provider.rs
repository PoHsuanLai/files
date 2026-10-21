//! Files as an app of docket: each action reads its call into an [`Ask`], hands it to the window
//! and shapes the answer.

use std::collections::BTreeSet;
use std::path::PathBuf;
use std::sync::Arc;

use docket_client::{ContextSource, IntentProvider, SummonTarget};
use docket_core::{
    AppRefusal, ContextScope, ContextSnapshot, EntityRef, FailText, Follow, Here, Hit, Invocation,
    LabelText, Outcome, Preview, Selection, SuggestAsk, SummonAnswer, SummonOrigin, SummonSerial,
    TextTarget, UndoFault, UndoToken, Undoable, ValidManifest, Value, Visible, WindowPrivacy,
};
use files_core::{Location, OpKind};
use porter_core::{AppName, Count, DataClass, SpaceId};
use prov::{Actor, Confidentiality, EntityId, Integrity, Label, Labelled, Source};

use super::args::ask_of;
use super::ask::{Ask, Reply, Undoing, Window};
use crate::error::Refusal;

/// Answers the router's calls from the window.
pub(super) struct Files<W> {
    pub(super) manifest: ValidManifest,
    pub(super) app: AppName,
    pub(super) window: Arc<W>,
}

/// What the router is told when the window would not.
fn refusal(why: Refusal) -> AppRefusal {
    match why {
        Refusal::Closed => AppRefusal::Busy,
        Refusal::Gone => AppRefusal::Failed(FailText(why.to_string())),
        Refusal::Failed(words) => AppRefusal::Failed(FailText(words)),
    }
}

impl<W: Window> IntentProvider for Files<W> {
    fn manifest(&self) -> &ValidManifest {
        &self.manifest
    }

    async fn perform(&self, call: Invocation) -> Result<Outcome, AppRefusal> {
        let ask = ask_of(&call)?;
        let reply = self.window.ask(ask.clone()).await.map_err(refusal)?;
        let shape = Shape {
            app: &self.app,
            space: &call.space,
        };
        shape.outcome(&ask, reply)
    }

    async fn dry_run(&self, _call: Invocation) -> Result<Preview, AppRefusal> {
        Err(AppRefusal::Unsupported)
    }

    async fn undo(&self, token: UndoToken, _actor: Actor) -> Result<(), UndoFault> {
        let Some(undoing) = Undoing::parse(token.as_str()) else {
            return Err(UndoFault::Gone);
        };
        match self.window.ask(Ask::Undo(undoing)).await {
            Ok(_) => Ok(()),
            Err(Refusal::Closed) => Err(UndoFault::AppUnavailable),
            Err(Refusal::Gone | Refusal::Failed(_)) => Err(UndoFault::Gone),
        }
    }

    async fn search(&self, _text: &str) -> Vec<Hit> {
        Vec::new()
    }

    async fn preview(&self, _id: &EntityId) -> Preview {
        Preview::None
    }

    async fn suggest(&self, _ask: SuggestAsk) -> Vec<EntityRef> {
        Vec::new()
    }
}

/// What the router is told of the window. Which items are selected is not here (the snapshot has
/// no way to say it without the window answering); an agent reads them with `files.selection`,
/// which taints its session.
pub(super) struct Snapshot {
    pub(super) app: AppName,
}

impl ContextSource for Snapshot {
    fn snapshot(&self, _scope: ContextScope) -> ContextSnapshot {
        let own = Label {
            integrity: Integrity::Trusted,
            confidentiality: Confidentiality::Public,
            classes: BTreeSet::from([DataClass::AppOwn]),
            sources: BTreeSet::from([Source::App(self.app.clone())]),
        };
        ContextSnapshot {
            app: self.app.clone(),
            window: Labelled::new("Files".to_owned(), own),
            here: Here::Nowhere,
            selection: Selection::Nothing,
            visible: Visible {
                kind: None,
                items: Vec::new(),
                total: Count(0),
            },
            text_target: TextTarget::None,
            privacy: WindowPrivacy::Normal,
        }
    }
}

/// Files has no prompt field for the companion to summon into.
pub(super) struct NoPrompt;

impl SummonTarget for NoPrompt {
    fn summon(&self, _serial: SummonSerial, _origin: SummonOrigin) -> SummonAnswer {
        SummonAnswer::Declined
    }
}

/// Builds outcomes with the labels this call's answers carry.
struct Shape<'a> {
    app: &'a AppName,
    space: &'a SpaceId,
}

fn items_words(count: usize, one: &str, many: &str) -> String {
    match count {
        1 => one.to_owned(),
        count => many.replace("{n}", &count.to_string()),
    }
}

impl Shape<'_> {
    fn outcome(&self, ask: &Ask, reply: Reply) -> Result<Outcome, AppRefusal> {
        match (ask, reply) {
            (Ask::Open(_), Reply::Done) => Ok(self.said("Showed it in Files", None)),
            (Ask::Trash(items), Reply::Done) => {
                let words = items_words(
                    items.len(),
                    "Moved 1 item to the Trash",
                    "Moved {n} items to the Trash",
                );
                Ok(self.said(&words, self.undo(OpKind::Trash, items.len())))
            }
            (Ask::NewFolder { .. }, Reply::Done) => {
                Ok(self.said("Made a folder", self.undo(OpKind::NewFolder, 1)))
            }
            (Ask::Copy { sources, .. }, Reply::Done) => {
                let words = items_words(sources.len(), "Copying 1 item", "Copying {n} items");
                Ok(self.said(&words, self.undo(OpKind::Copy, sources.len())))
            }
            (Ask::Move { sources, .. }, Reply::Done) => {
                let words = items_words(sources.len(), "Moving 1 item", "Moving {n} items");
                Ok(self.said(&words, self.undo(OpKind::Move, sources.len())))
            }
            (Ask::EmptyTrash, Reply::Done) => {
                Ok(self.said("Asked whether to empty the Trash", None))
            }
            (Ask::Selection, Reply::Items(paths)) => Ok(self.read(lines(&paths))),
            (Ask::CurrentFolder, Reply::Place(place)) => Ok(self.read(place_text(&place))),
            (Ask::Undo(_), Reply::Done) => Ok(self.said("Undone", None)),
            (
                Ask::Open(_)
                | Ask::Trash(_)
                | Ask::NewFolder { .. }
                | Ask::Copy { .. }
                | Ask::Move { .. }
                | Ask::EmptyTrash
                | Ask::Selection
                | Ask::CurrentFolder
                | Ask::Undo(_),
                _,
            ) => Err(AppRefusal::Failed(FailText(
                "Files answered something else".to_owned(),
            ))),
        }
    }

    /// File names the window read out: somebody's words, private to the Space they were read in.
    fn read(&self, text: String) -> Outcome {
        let label = Label::untrusted(Source::File, DataClass::Files, self.space.clone());
        Outcome {
            value: Some(Labelled::new(Value::Text(text), label)),
            said: None,
            show: Preview::None,
            undo: Undoable::No,
            follow: Follow::Nothing,
        }
    }

    fn said(&self, words: &str, undo: Option<UndoToken>) -> Outcome {
        Outcome {
            value: None,
            said: LabelText::parse(words).ok(),
            show: Preview::None,
            undo: undo.map_or(Undoable::No, Undoable::Yes),
            follow: Follow::Nothing,
        }
    }

    /// The token that takes back the change just queued.
    fn undo(&self, kind: OpKind, count: usize) -> Option<UndoToken> {
        let token = Undoing { kind, count }.token()?;
        UndoToken::parse(&token).ok()
    }
}

/// Paths one to a line.
fn lines(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| path.to_string_lossy())
        .collect::<Vec<_>>()
        .join("\n")
}

/// A place as the line an agent reads.
fn place_text(place: &Location) -> String {
    match place {
        Location::Folder(folder) => folder.as_path().to_string_lossy().into_owned(),
        Location::Trash | _ => "trash:///".to_owned(),
    }
}
