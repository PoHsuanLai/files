//! Work in progress, questions waiting for an answer, and what the workers report.

use files_core::{Conflict, OpFailure, Progress, Summary, TrashedItem};

/// How a drop was meant, as the window says it (the model never reads modifier keys).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DropHow {
    /// The usual: a move within one disk, a copy across disks.
    Natural,
    /// A copy, whatever the disks (the person held the copy key).
    Copy,
}

/// The change being made now, if any.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Work {
    /// Nothing is being changed.
    #[default]
    Idle,
    /// A change is under way.
    Running {
        /// What it is about.
        summary: Summary,
        /// How far it has come.
        progress: Progress,
    },
}

/// Something the window must ask before it goes on.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Question {
    /// Nothing is being asked.
    #[default]
    None,
    /// Whether to erase everything in the Trash for good.
    EmptyTrash,
    /// Whether to erase these items of the Trash for good.
    DeleteForGood(Vec<TrashedItem>),
    /// A name is taken: keep both, replace, skip or stop.
    Conflict(Conflict),
}

/// How a piece of work ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum End {
    /// All of it was done.
    Done,
    /// It was stopped; what was done stays done.
    Stopped,
    /// It failed at some point.
    Failed(OpFailure),
}

/// A piece of work that ended.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Outcome {
    /// What it was about.
    pub summary: Summary,
    /// How it ended.
    pub end: End,
    /// What undoing would take back now, if anything.
    pub undo: Option<Summary>,
}

impl Outcome {
    /// `summary` ended as `end`, leaving `undo` to take back.
    pub fn new(summary: Summary, end: End, undo: Option<Summary>) -> Outcome {
        Outcome { summary, end, undo }
    }
}

/// What the workers tell the browser about a change.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
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
