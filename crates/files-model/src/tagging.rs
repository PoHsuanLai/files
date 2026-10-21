//! Tagging the selection with a colour.

use files_core::TagColour;

use crate::browser::Browser;
use crate::effect::{Effect, Mark};

type Stepped = (Browser, Vec<Effect>);

/// How much of the selection carries a tag.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Held {
    /// Nothing selected, or none of it has the tag.
    Nothing,
    /// Some of the selected items have it.
    Part,
    /// Every selected item has it.
    Every,
}

impl Browser {
    /// How much of the selection carries the tag of `colour`, for the check mark in the menu.
    pub fn held(&self, colour: TagColour) -> Held {
        let rows = self.selected_rows();
        let with = rows.iter().filter(|(row, _)| row.tags.has(colour)).count();
        match (with, rows.len()) {
            (0, _) => Held::Nothing,
            (with, all) if with == all => Held::Every,
            (_, _) => Held::Part,
        }
    }

    /// Put the tag of `colour` on the selection, or take it off when every selected item has it.
    pub(crate) fn tag(self, colour: TagColour) -> Stepped {
        let how = match self.held(colour) {
            Held::Every => Mark::Remove,
            Held::Nothing | Held::Part => Mark::Add,
        };
        let items: Vec<_> = self
            .selected_rows()
            .into_iter()
            .map(|(_, item)| item)
            .collect();
        if items.is_empty() {
            return (self, Vec::new());
        }
        let effect = Effect::Tag { items, colour, how };
        (self, vec![effect])
    }
}
