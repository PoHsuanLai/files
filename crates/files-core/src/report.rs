//! What a piece of work reports as it goes and when it ends.

use crate::{Conflict, Location, OpFailure, Progress, Summary};

/// How a piece of work ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Finish {
    /// All of it was done.
    Done,
    /// It was stopped (cancelled, or answered Stop); what was done stays done.
    Stopped,
    /// It failed at some point; what was done before stays done.
    Failed(OpFailure),
}

/// A piece of work that ended.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Outcome {
    /// What the work was about.
    pub summary: Summary,
    /// How it ended.
    pub finish: Finish,
    /// The places that now hold something different.
    pub touched: Vec<Location>,
    /// What undoing would take back now, if anything.
    pub undo: Option<Summary>,
}

impl Outcome {
    /// `summary` ended as `finish`, leaving `undo` to take back; it touched no place yet.
    pub fn new(summary: Summary, finish: Finish, undo: Option<Summary>) -> Outcome {
        Outcome {
            summary,
            finish,
            touched: Vec::new(),
            undo,
        }
    }

    /// The same outcome, having changed `touched`.
    pub fn touching(self, touched: Vec<Location>) -> Outcome {
        Outcome { touched, ..self }
    }
}

/// What the workers tell about a change.
#[derive(Debug, Clone, PartialEq)]
pub enum OpEvent {
    /// A change began.
    Started(Summary),
    /// How far it has come.
    Progress(Progress),
    /// It is waiting for an answer to a taken name.
    Asking(Conflict),
    /// It ended.
    Finished(Outcome),
}
