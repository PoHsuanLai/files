//! What a piece of work did.

use files_core::{Location, OpFailure, Summary};

use crate::journal::Inverse;

/// How a piece of work ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Finish {
    /// All of it was done.
    Done,
    /// It was stopped (cancelled, or answered Stop); what was done stays done.
    Stopped,
    /// It failed at some point; what was done before stays done.
    Failed(OpFailure),
}

/// A piece of work that has ended: what it was, how it ended, where it changed things, and how
/// to undo what it did.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Ran {
    /// What the work was about.
    pub summary: Summary,
    /// How it ended.
    pub finish: Finish,
    /// The places that now hold something different: the folders it changed, and the Trash.
    pub touched: Vec<Location>,
    pub(crate) steps: Vec<Inverse>,
}

impl Ran {
    /// Whether anything was done that [`Journal::undo`](crate::Journal::undo) can take back.
    pub fn is_undoable(&self) -> bool {
        !self.steps.is_empty()
    }
}
