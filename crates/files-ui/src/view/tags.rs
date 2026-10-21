//! Tags as the window draws them: a colour's dot, from quire's chip and `--tag-*` tokens.

use dioxus::prelude::*;
use ds::components::controls::chip::{Chip, ChipVariant};
use files_core::{Tag, TagColour, Tags};

/// The CSS colour of a tag: quire's `--tag-*` token. quire has no gray tag, so Gray is the faint
/// ink.
pub(super) fn tag_css(colour: TagColour) -> &'static str {
    match colour {
        TagColour::Red => "var(--tag-red)",
        TagColour::Orange => "var(--tag-orange)",
        TagColour::Yellow => "var(--tag-yellow)",
        TagColour::Green => "var(--tag-green)",
        TagColour::Blue => "var(--tag-blue)",
        TagColour::Purple => "var(--tag-purple)",
        TagColour::Gray | _ => "var(--ink-faint)",
    }
}

/// A dot of `colour`: a chip with a dot and no words.
pub(super) fn dot(colour: TagColour) -> Element {
    rsx! {
        Chip {
            variant: ChipVariant::Neutral,
            text: String::new(),
            dot: Some(tag_css(colour).to_owned()),
        }
    }
}

/// What a tag is drawn as: its colour's dot, or a chip with its name when it is no colour.
fn mark(tag: &Tag) -> Element {
    match tag.colour() {
        Some(colour) => dot(colour),
        None => rsx! {
            Chip { variant: ChipVariant::Neutral, text: tag.name().to_owned() }
        },
    }
}

/// The tags of a row, after its name.
pub(super) fn marks(tags: &Tags) -> Element {
    rsx! {
        for (at, tag) in tags.iter().enumerate() {
            span { key: "{at}", {mark(tag)} }
        }
    }
}
