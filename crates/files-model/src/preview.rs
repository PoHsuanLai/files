//! The preview pane (Finder's Show Preview): whether it is up, and what it is about.
//!
//! The pane belongs to the window, so the tabs share whether it is up. What it shows follows the
//! selection: one item gets its preview and its facts, several get a count and a total size, none
//! gets nothing. The browser asks for the preview and the facts of the one item whenever the pane
//! is up and that item changes ([`Effect::Look`] and [`Effect::ReadDetails`]); an answer for an item
//! that is no longer the one is ignored.

use files_core::{ByteSize, Entry, EntryKind, Info, ItemRef, LinkTo};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::error::ListFailure;

type Stepped = (Browser, Vec<Effect>);

/// Whether the preview pane is up.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Pane {
    /// Not up.
    #[default]
    Hidden,
    /// Up, beside the folder.
    Shown,
}

impl Pane {
    /// The other state: what Show Preview or Hide Preview asks for.
    pub fn flipped(self) -> Pane {
        match self {
            Pane::Hidden => Pane::Shown,
            Pane::Shown => Pane::Hidden,
        }
    }
}

/// What the pane is about, from the selection.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Preview {
    /// The pane is not up.
    Hidden,
    /// The pane is up and nothing is selected (or the selection has no place on disk).
    Nothing,
    /// One item is selected: its preview and its facts.
    One(ItemRef),
    /// Several items are selected: how many, and how much they weigh together (folders count as
    /// no size).
    Many {
        /// How many items.
        count: usize,
        /// The sum of the sizes of the files among them.
        size: ByteSize,
    },
}

/// The bytes a file takes; folders and the like take none.
fn weight(entry: &Entry) -> u64 {
    match &entry.kind {
        EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) => size.0,
        EntryKind::Folder | EntryKind::Link(_) | EntryKind::Special(_) | _ => 0,
    }
}

impl Browser {
    /// Whether the preview pane is up.
    pub fn pane(&self) -> Pane {
        self.pane
    }

    /// What the pane is about now.
    pub fn preview(&self) -> Preview {
        if self.pane == Pane::Hidden {
            return Preview::Hidden;
        }
        let rows = self.selected_rows();
        match rows.as_slice() {
            [] => Preview::Nothing,
            [(_, item)] => Preview::One(item.clone()),
            many => Preview::Many {
                count: many.len(),
                size: ByteSize(many.iter().map(|(row, _)| weight(row)).sum()),
            },
        }
    }

    /// The item the pane shows while it is up and exactly one item is selected: the one whose
    /// preview and facts are asked for.
    pub fn previewed_item(&self) -> Option<ItemRef> {
        match self.preview() {
            Preview::One(item) => Some(item),
            Preview::Hidden | Preview::Nothing | Preview::Many { .. } => None,
        }
    }

    /// The row of the item the pane shows (see [`Browser::previewed_item`]).
    pub fn previewed_entry(&self) -> Option<&Entry> {
        match self.selected_rows().as_slice() {
            [(row, _)] if self.pane == Pane::Shown => Some(*row),
            _ => None,
        }
    }

    /// The facts read for the item the pane shows, once they have arrived.
    pub fn details(&self) -> Option<&Info> {
        self.details.as_ref()
    }

    /// Show the pane, or put it away (⇧⌘P).
    pub(crate) fn toggle_pane(self) -> Stepped {
        let pane = self.pane.flipped();
        (Browser { pane, ..self }, Vec::new())
    }

    /// The answer to [`Effect::ReadDetails`]. One for an item the pane is not about is ignored,
    /// and so is a failure: the pane then shows what the row already says.
    pub(crate) fn details_read(self, item: ItemRef, result: Result<Info, ListFailure>) -> Stepped {
        let current = self.previewed_item();
        match result {
            Ok(info) if current.as_ref() == Some(&item) => (
                Browser {
                    details: Some(info),
                    ..self
                },
                Vec::new(),
            ),
            Ok(_) | Err(_) => (self, Vec::new()),
        }
    }

    /// Brings the pane's asks in line with the selection: when the item it shows is not the one
    /// it showed (`before`), the facts of the old one go and the preview and facts of the new one
    /// are asked for.
    pub(crate) fn previewing(self, before: Option<ItemRef>) -> Stepped {
        let now = self.previewed_item();
        if now == before {
            return (self, Vec::new());
        }
        let effects = now
            .into_iter()
            .flat_map(|item| [Effect::Look(item.clone()), Effect::ReadDetails(item)])
            .collect();
        (
            Browser {
                details: None,
                ..self
            },
            effects,
        )
    }
}
