//! The previews the window holds, by the item each is of.
//!
//! Quick Look and the preview pane ask one worker for previews, and an answer says which item it
//! is of. Whoever draws reads the preview of the item it is about and nothing else, so an answer
//! for an item that is no longer shown is never drawn, and the two never take each other's.

use std::sync::Arc;

use anyview_peek::AnyPeeked;
use files_core::ItemRef;

/// How many previews are kept: the two views (Quick Look and the pane) and a little more.
const KEPT: usize = 4;

/// The newest previews received, each of the item it was asked for.
#[derive(Debug, Clone, Default)]
pub struct Looks {
    /// Oldest first.
    held: Vec<(ItemRef, Arc<AnyPeeked>)>,
}

impl Looks {
    /// The preview of `item`, if one has arrived and is still kept.
    pub fn of(&self, item: &ItemRef) -> Option<Arc<AnyPeeked>> {
        self.held
            .iter()
            .find(|(held, _)| held == item)
            .map(|(_, card)| Arc::clone(card))
    }

    /// Keeps `card` as the preview of `item`, in place of an earlier one of the same item and
    /// pushing out the oldest of the others past the limit.
    pub fn put(&mut self, item: ItemRef, card: Arc<AnyPeeked>) {
        self.held.retain(|(held, _)| *held != item);
        self.held.push((item, card));
        if self.held.len() > KEPT {
            self.held.remove(0);
        }
    }
}
