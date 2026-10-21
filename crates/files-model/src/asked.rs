//! Changes another program asked for: items named by where they are, not by the folder shown.
//!
//! They become the same plans as the person's own changes, so they run on the same queue, show the
//! same progress, can be undone the same way, and a taken name asks the same question.

use files_core::{FileName, FolderPath, ItemRef, OpPlan, Split, Transfer, keep_both};

use crate::browser::Browser;
use crate::changes::NEW_FOLDER;
use crate::effect::Effect;
use crate::ops::Question;

type Stepped = (Browser, Vec<Effect>);

/// What to do with items brought to a folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Bring {
    /// Leave the originals where they are.
    Copy,
    /// Take them out of where they were.
    Move,
}

impl Browser {
    /// Move `items` to the Trash.
    pub(crate) fn trash_items(self, items: Vec<ItemRef>) -> Stepped {
        if items.is_empty() {
            return (self, Vec::new());
        }
        self.run(OpPlan::Trash { items })
    }

    /// Make the folder `name` in `parent`; with no name, `untitled folder`, numbered past what the
    /// shown folder holds when `parent` is the shown folder.
    pub(crate) fn make_folder(self, parent: FolderPath, name: Option<FileName>) -> Stepped {
        match name.or_else(|| self.fresh_name(&parent)) {
            Some(name) => self.run(OpPlan::NewFolder { parent, name }),
            None => (self, Vec::new()),
        }
    }

    /// `untitled folder`, or the next free `untitled folder 2`, as far as the shown folder says.
    fn fresh_name(&self, parent: &FolderPath) -> Option<FileName> {
        let base = FileName::new(NEW_FOLDER).ok()?;
        if self.folder() != Some(parent) {
            return Some(base);
        }
        let entries = self.listing.entries();
        keep_both(&base, Split::Whole, |name| {
            entries.iter().any(|entry| entry.name == *name)
        })
        .ok()
    }

    /// Copy or move `sources` into `destination`.
    pub(crate) fn bring(
        self,
        sources: Vec<ItemRef>,
        destination: FolderPath,
        how: Bring,
    ) -> Stepped {
        if sources.is_empty() {
            return (self, Vec::new());
        }
        let transfer = Transfer::new(sources, destination);
        match how {
            Bring::Copy => self.run(OpPlan::Copy(transfer)),
            Bring::Move => self.run(OpPlan::Move(transfer)),
        }
    }

    /// Put up the question whether to empty the Trash, wherever the window is.
    pub(crate) fn ask_empty_trash(self) -> Stepped {
        self.asking(Question::EmptyTrash)
    }
}
