//! Looking for the files that carry one colour, folder by folder.

use files_core::{Entry, EntryKind, FolderPath, ItemRef, TagColour};

use crate::list::Lister;

/// The most entries a walk looks at before it stops and says so.
pub const WALK_CAP: usize = 20_000;

/// One file found with the colour asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Found {
    /// Where it is.
    pub item: ItemRef,
    /// What the listing said of it, with its tags.
    pub entry: Entry,
}

/// What the caller of a walk wants after a batch of finds.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Go on looking.
    Go,
    /// Stop now (the results are no longer wanted).
    Stop,
}

/// How a walk ended.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Walked {
    /// Every folder was looked through.
    Finished,
    /// The walk stopped at the cap on entries; there may be more.
    Capped,
    /// The caller stopped it.
    Stopped,
}

/// A walk for one colour: where to start, which colour, and how many entries to look at.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct TagWalk {
    root: FolderPath,
    colour: TagColour,
    cap: usize,
}

impl TagWalk {
    /// A walk of `root` and the folders under it for entries tagged `colour`, capped at
    /// [`WALK_CAP`] entries.
    pub fn new(root: FolderPath, colour: TagColour) -> TagWalk {
        TagWalk {
            root,
            colour,
            cap: WALK_CAP,
        }
    }

    /// The same walk looking at no more than `cap` entries.
    pub fn with_cap(mut self, cap: usize) -> TagWalk {
        self.cap = cap;
        self
    }

    /// Looks through the folders, handing each folder's finds to `found` as they come; `found`
    /// answers [`Flow::Stop`] to stop the walk.
    ///
    /// Links are not followed, a folder that can't be read is passed over, and a folder whose
    /// name starts with a dot is not entered (the walk would drown in caches).
    pub fn run(&self, lister: &dyn Lister, found: &mut dyn FnMut(Vec<Found>) -> Flow) -> Walked {
        let (colour, cap) = (self.colour, self.cap);
        let mut pending = vec![self.root.clone()];
        let mut seen = 0usize;
        while let Some(folder) = pending.pop() {
            let Ok(entries) = lister.list(&folder) else {
                continue;
            };
            let mut finds = Vec::new();
            let mut capped = false;
            for entry in entries {
                if seen >= cap {
                    capped = true;
                    break;
                }
                seen += 1;
                if matches!(entry.kind, EntryKind::Folder)
                    && entry.visibility() == files_core::Visibility::Shown
                {
                    pending.push(folder.join(&entry.name));
                }
                if entry.tags.has(colour) {
                    let item = ItemRef::new(folder.clone(), entry.name.clone());
                    finds.push(Found { item, entry });
                }
            }
            if !finds.is_empty() && found(finds) == Flow::Stop {
                return Walked::Stopped;
            }
            if capped {
                return Walked::Capped;
            }
        }
        Walked::Finished
    }
}
