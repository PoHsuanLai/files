//! A tile that wants its thumbnail for as long as it is drawn.

use std::cell::RefCell;
use std::rc::Rc;

use dioxus::prelude::*;

use crate::edge::Edge;
use crate::state::Browse;
use crate::thumbs::ThumbKey;

/// A wish held with the edge: it is made on creation and withdrawn when the holder is dropped.
struct Wish {
    edge: Rc<Edge>,
    key: ThumbKey,
}

impl Wish {
    fn make(edge: Rc<Edge>, key: ThumbKey) -> Wish {
        edge.want_thumbnail(key.clone());
        Wish { edge, key }
    }
}

impl Drop for Wish {
    fn drop(&mut self) {
        self.edge.unwant_thumbnail(&self.key);
    }
}

/// Draws nothing. While it is part of a tile (quire mounts only the tiles near the viewport) the
/// thumbnail `thumb` is wanted; when the tile scrolls away, is replaced by one of another version
/// or gets its picture, the wish ends and a thumbnail not yet made for it is not made.
#[component]
pub(super) fn Wanted(browse: Browse, thumb: ThumbKey) -> Element {
    let held = use_hook(|| Rc::new(RefCell::new(None::<Wish>)));
    use_effect(use_reactive!(|thumb| {
        // Replacing the wish withdraws the old one after the new one is made.
        *held.borrow_mut() = Some(Wish::make(browse.edge(), thumb));
    }));
    rsx! {}
}
