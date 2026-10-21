//! Plans of work: what the person asked to change, as data for an executor.

use crate::{FileName, FolderPath, ItemRef, TrashedItem};

/// Items to bring to a folder.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Transfer {
    /// What to bring, in order.
    pub sources: Vec<ItemRef>,
    /// The folder to bring it to.
    pub destination: FolderPath,
}

impl Transfer {
    /// `sources` to `destination`.
    pub fn new(sources: Vec<ItemRef>, destination: FolderPath) -> Transfer {
        Transfer {
            sources,
            destination,
        }
    }
}

/// One change to the files, asked for as a whole.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum OpPlan {
    /// Copy items to a folder.
    Copy(Transfer),
    /// Move items to a folder.
    Move(Transfer),
    /// Bring items to a folder the way a drag does: a move when the folder is on the same disk as
    /// the items, a copy when it is on another.
    Drag(Transfer),
    /// Make a new empty folder.
    NewFolder {
        /// The folder to make it in.
        parent: FolderPath,
        /// Its name.
        name: FileName,
    },
    /// Give an item another name in the same folder.
    Rename {
        /// The item.
        item: ItemRef,
        /// Its new name.
        to: FileName,
    },
    /// Move items to the Trash.
    Trash {
        /// The items.
        items: Vec<ItemRef>,
    },
    /// Move items from the Trash back to where they were.
    PutBack {
        /// The items.
        items: Vec<TrashedItem>,
    },
    /// Erase items in the Trash for good.
    DeleteForGood {
        /// The items.
        items: Vec<TrashedItem>,
    },
    /// Erase everything in the Trash for good.
    EmptyTrash,
}

/// What kind of change a plan is, for the words said about it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum OpKind {
    /// Copying.
    Copy,
    /// Moving.
    Move,
    /// Making a folder.
    NewFolder,
    /// Renaming.
    Rename,
    /// Moving to the Trash.
    Trash,
    /// Putting back from the Trash.
    PutBack,
    /// Erasing for good.
    Erase,
}

/// What a plan is about, for saying so: its kind, how many items and the first one's name.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Summary {
    /// What kind of change.
    pub kind: OpKind,
    /// How many items it covers.
    pub count: usize,
    /// The name of the first item, if there is one.
    pub first: Option<FileName>,
}

impl Summary {
    /// A summary of `count` items of `kind`, the first called `first`.
    pub fn new(kind: OpKind, count: usize, first: Option<FileName>) -> Summary {
        Summary { kind, count, first }
    }
}

impl OpPlan {
    /// What the plan is about.
    pub fn summary(&self) -> Summary {
        let of_refs = |kind, items: &[ItemRef]| {
            Summary::new(kind, items.len(), items.first().map(|i| i.name.clone()))
        };
        let of_trashed = |kind, items: &[TrashedItem]| {
            let first = items.first().map(|i| i.original.name.clone());
            Summary::new(kind, items.len(), first)
        };
        match self {
            OpPlan::Copy(t) => of_refs(OpKind::Copy, &t.sources),
            OpPlan::Move(t) | OpPlan::Drag(t) => of_refs(OpKind::Move, &t.sources),
            OpPlan::NewFolder { name, .. } => {
                Summary::new(OpKind::NewFolder, 1, Some(name.clone()))
            }
            OpPlan::Rename { to, .. } => Summary::new(OpKind::Rename, 1, Some(to.clone())),
            OpPlan::Trash { items } => of_refs(OpKind::Trash, items),
            OpPlan::PutBack { items } => of_trashed(OpKind::PutBack, items),
            OpPlan::DeleteForGood { items } => of_trashed(OpKind::Erase, items),
            OpPlan::EmptyTrash => Summary::new(OpKind::Erase, 0, None),
        }
    }
}
