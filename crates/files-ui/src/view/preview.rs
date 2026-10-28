//! The preview pane (Finder's Show Preview): quire's `PreviewPane` beside the folder, holding
//! the card anyview draws for Quick Look over the selected item's facts. Several items are a
//! count and their total size; none is "No Selection".

use anyview_core::FactLabel;
use anyview_peek::{AnyPeeked, Card, Part, Parts};
use dioxus::prelude::*;
use ds::components::content::label::LabelStyle;
use ds::components::lists::preview::content::PaneContent;
use ds::prelude::{Column, ColumnAlign, Fact, FactList, Icon, IconSource, Label, PreviewPane};
use files_core::{Entry, Info, ItemRef};
use files_model::Preview;

use crate::edge::Zone;
use crate::state::Browse;
use crate::words::{many_words, pane_rows, size_words};

/// What anyview reports of a picture's size, when it reports one: the card's own row.
fn dimensions_of(peeked: &AnyPeeked) -> Option<String> {
    peeked
        .facts
        .rows()
        .iter()
        .find(|row| row.label == FactLabel::Dimensions)
        .map(|row| row.value.as_str().to_owned())
}

/// A pane about no one item: its glyph, a title and rows.
fn about(title: String, rows: Vec<(String, String)>) -> PaneContent {
    PaneContent::Facts {
        icon: IconSource::Glyph(Icon::File),
        title,
        rows,
    }
}

/// The pane of one item: the card (or its name until the card arrives) over its facts.
fn one(
    browse: Browse,
    zone: Zone,
    item: &ItemRef,
    entry: &Entry,
    info: Option<&Info>,
) -> PaneContent {
    let name = item.name.display().into_owned();
    let card = browse.looked.read().of(item);
    let dimensions = card.as_deref().and_then(dimensions_of);
    let facts: Vec<Fact> = pane_rows(entry, info, dimensions, |at| zone.words(at))
        .into_iter()
        .map(|row| Fact::new(row.label, row.value))
        .collect();
    let head = match card {
        Some(peeked) => rsx! {
            Card { peeked, parts: Parts::of([Part::Media, Part::Title]) }
        },
        None => rsx! {
            Label { text: name.clone(), style: LabelStyle::Title }
        },
    };
    PaneContent::Slot {
        label: name,
        element: rsx! {
            Column { align: ColumnAlign::Center,                 {head}
                FactList { facts }
            }
        },
    }
}

/// The preview pane, while it is up.
#[component]
pub(super) fn PreviewColumn(browse: Browse, zone: Zone) -> Element {
    let (preview, entry, details) = {
        let state = browse.state.read();
        (
            state.preview(),
            state.previewed_entry().cloned(),
            state.details().cloned(),
        )
    };
    let content = match (preview, entry) {
        (Preview::Nothing, _) => about("No Selection".to_owned(), Vec::new()),
        (Preview::Many { count, size }, _) => about(
            many_words(count),
            vec![("Size".to_owned(), size_words(size))],
        ),
        (Preview::One(item), Some(entry)) => {
            // Facts read for another item (the selection moved on) are not this one's.
            let info = details.filter(|info| info.item() == item);
            one(browse, zone, &item, &entry, info.as_ref())
        }
        // Hidden, or a state files-model adds later: nothing is drawn.
        _ => return rsx! {},
    };
    rsx! {
        PreviewPane { content }
    }
}
