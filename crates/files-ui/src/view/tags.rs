//! Tags as the window draws them: a colour's dot, from quire's chip and `--tag-*` tokens.

use dioxus::prelude::*;
use ds::components::controls::chip::{Chip, ChipVariant};
use ds::components::controls::chip_dot::ChipDot;
use ds::style::tokens::tag_colour::TagColour as Hue;
use files_core::{Tag, TagColour, Tags};

/// quire's tag colour for a tag's: painted from its `--tag-*` token.
fn hue(colour: TagColour) -> Hue {
    match colour {
        TagColour::Red => Hue::Red,
        TagColour::Orange => Hue::Orange,
        TagColour::Yellow => Hue::Yellow,
        TagColour::Green => Hue::Green,
        TagColour::Blue => Hue::Blue,
        TagColour::Purple => Hue::Purple,
        TagColour::Gray | _ => Hue::Gray,
    }
}

/// A dot of `colour`: a chip with a dot and no words.
pub(super) fn dot(colour: TagColour) -> Element {
    rsx! {
        Chip {
            variant: ChipVariant::Neutral,
            text: String::new(),
            dot: ChipDot::Tag(hue(colour)),
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
