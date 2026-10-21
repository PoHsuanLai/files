//! The undo journal: for each finished piece of work, the steps that take it back.

use files_core::{FileName, ItemRef, Summary, TrashedItem};

use crate::exec::Executor;
use crate::host::Host;
use crate::ran::Ran;

/// The most pieces of work the journal remembers.
const KEPT: usize = 100;

/// One step that takes back part of a piece of work.
#[derive(Debug, Clone, PartialEq)]
pub(crate) enum Inverse {
    /// Move these items to the Trash: undoes a copy, or a put back.
    Trash(Vec<ItemRef>),
    /// Move `now` back to where `back` is: undoes a move.
    MoveBack {
        /// Where the item is now.
        now: ItemRef,
        /// Where it was.
        back: ItemRef,
    },
    /// Move this folder to the Trash if it is still empty: undoes a new folder.
    RemoveEmpty(ItemRef),
    /// Give `now` its old name again: undoes a rename.
    RenameBack {
        /// The item as it is called now.
        now: ItemRef,
        /// The name it had.
        was: FileName,
    },
    /// Put these items back from the Trash: undoes a move to the Trash, or a replace.
    PutBack(Vec<TrashedItem>),
}

/// What was done and how to undo it.
#[derive(Debug, Clone, PartialEq)]
struct Entry {
    summary: Summary,
    steps: Vec<Inverse>,
}

/// The journal of finished work, newest last. Each piece of work that changed something is one
/// entry; [`Journal::undo`] takes back the newest.
#[derive(Debug, Clone, Default)]
pub struct Journal {
    entries: Vec<Entry>,
}

impl Journal {
    /// An empty journal.
    pub fn new() -> Journal {
        Journal::default()
    }

    /// Remembers `ran` if it changed anything (a stopped or failed piece of work that got part
    /// of the way is remembered for the part it did).
    pub fn record(&mut self, ran: &Ran) {
        if ran.steps.is_empty() {
            return;
        }
        self.entries.push(Entry {
            summary: ran.summary.clone(),
            steps: ran.steps.clone(),
        });
        if self.entries.len() > KEPT {
            self.entries.remove(0);
        }
    }

    /// What [`Journal::undo`] would take back next.
    pub fn last(&self) -> Option<&Summary> {
        self.entries.last().map(|entry| &entry.summary)
    }

    /// How many pieces of work can be taken back.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there is nothing to take back.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// Takes back the newest piece of work, with `exec`, and says what happened; `None` when the
    /// journal is empty. The entry is gone from the journal whether or not every step worked:
    /// the steps that did not are reported in the result's `finish`.
    pub fn undo(&mut self, exec: &Executor, host: &mut dyn Host) -> Option<Ran> {
        let entry = self.entries.pop()?;
        Some(exec.undo(entry.summary, entry.steps, host))
    }
}
