//! What the assistant and the command line may ask of the window, as typed data, and the window's
//! side of it.

use std::future::Future;
use std::path::PathBuf;

use files_core::{FileName, FolderPath, ItemRef, Location, OpKind};

use crate::error::Refusal;

/// One thing asked of Files. Paths are full paths; the window decides what a path is (a folder to
/// open, a file to point at).
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Ask {
    /// Show this file or folder.
    Open(PathBuf),
    /// Move these items to the Trash.
    Trash(Vec<ItemRef>),
    /// Make a folder in `parent`, called `name`, or `untitled folder` when no name is given.
    NewFolder {
        /// The folder to make it in.
        parent: FolderPath,
        /// Its name, when given.
        name: Option<FileName>,
    },
    /// Copy these items into a folder.
    Copy {
        /// What to copy.
        sources: Vec<ItemRef>,
        /// The folder to copy into.
        to: FolderPath,
    },
    /// Move these items into a folder.
    Move {
        /// What to move.
        sources: Vec<ItemRef>,
        /// The folder to move into.
        to: FolderPath,
    },
    /// Ask the person whether to empty the Trash; nothing is erased by this alone.
    EmptyTrash,
    /// Say which items are selected.
    Selection,
    /// Say which folder the window shows.
    CurrentFolder,
    /// Take back the newest change, when it is the one described.
    Undo(Undoing),
}

/// The change an undo token stands for: what kind and how many items. The window takes back the
/// newest change only if it is this one, so an undo never takes back something the person did
/// since.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Undoing {
    /// What kind of change.
    pub kind: OpKind,
    /// How many items it covered.
    pub count: usize,
}

/// The words a token spells a kind of change with.
const KINDS: [(OpKind, &str); 4] = [
    (OpKind::Trash, "trash"),
    (OpKind::Copy, "copy"),
    (OpKind::Move, "move"),
    (OpKind::NewFolder, "new_folder"),
];

impl Undoing {
    /// The token that names this change: `undo:trash:3`.
    pub fn token(&self) -> Option<String> {
        let word = KINDS
            .iter()
            .find(|(kind, _)| *kind == self.kind)
            .map(|(_, word)| *word)?;
        Some(format!("undo:{word}:{}", self.count))
    }

    /// The change a token names, when it is one of ours.
    pub fn parse(token: &str) -> Option<Undoing> {
        let rest = token.strip_prefix("undo:")?;
        let (word, count) = rest.split_once(':')?;
        let kind = KINDS
            .iter()
            .find(|(_, known)| *known == word)
            .map(|(kind, _)| *kind)?;
        Some(Undoing {
            kind,
            count: count.parse().ok()?,
        })
    }
}

/// What the window answers.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reply {
    /// It was done, or the person was asked, or the window was told.
    Done,
    /// The selected items, in shown order.
    Items(Vec<PathBuf>),
    /// The place the window shows.
    Place(Location),
}

/// The window, as the actions see it. The binary implements it over the window's request
/// channel; a test implements it with a fake.
pub trait Window: Send + Sync + 'static {
    /// Carries out `ask` and answers. Changes are queued on the window's own queue of work and
    /// answered once queued, not once done.
    fn ask(&self, ask: Ask) -> impl Future<Output = Result<Reply, Refusal>> + Send;
}
