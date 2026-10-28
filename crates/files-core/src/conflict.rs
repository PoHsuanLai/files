//! A name that is already taken where an item is going, and how the person settles it.

use crate::{FileName, FolderPath};

/// What the work was doing when it found the name taken.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum ConflictKind {
    /// Copying the item there.
    Copy,
    /// Moving the item there.
    Move,
}

/// A question the work is waiting on: something in `folder` already has the name `item`.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Conflict {
    /// The name that is taken.
    pub item: FileName,
    /// The folder it is taken in.
    pub folder: FolderPath,
    /// What was being done.
    pub kind: ConflictKind,
    /// How many more items are still to come after this one.
    pub rest: usize,
}

impl Conflict {
    /// The name `item` is taken in `folder` while doing `kind`, with `rest` items still to come.
    pub fn new(item: FileName, folder: FolderPath, kind: ConflictKind, rest: usize) -> Conflict {
        Conflict {
            item,
            folder,
            kind,
            rest,
        }
    }
}

/// What to do about a taken name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum Answer {
    /// Keep both: the new item gets another name (`name 2`).
    KeepBoth,
    /// The new item takes the old one's place; the old one goes to the Trash.
    Replace,
    /// Leave this item out and go on with the rest.
    Skip,
    /// Stop here; what is done stays done.
    Stop,
}

/// An answer and how far it reaches.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub struct Resolution {
    /// What to do.
    pub answer: Answer,
    /// Whether it also holds for every later item that finds its name taken, not only this one.
    pub to_rest: bool,
}

impl Resolution {
    /// `answer` for this item only.
    pub fn once(answer: Answer) -> Resolution {
        Resolution {
            answer,
            to_rest: false,
        }
    }

    /// `answer` for this item and every later conflict.
    pub fn for_rest(answer: Answer) -> Resolution {
        Resolution {
            answer,
            to_rest: true,
        }
    }
}
