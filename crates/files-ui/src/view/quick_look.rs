//! Quick Look: quire's `Peek` over the window, drawing anyview's card for the row the keyboard
//! is on. The card is the one anyview's own pane and mailo's attachments draw; a folder is its
//! icon, name and the number of items in it.

use anyview_peek::Card;
use dioxus::prelude::*;
use ds::components::app::peek::Peek;
use ds::components::content::label::LabelStyle;
use ds::prelude::{Column, ColumnAlign, ColumnExtent, Label};
use ds::style::appearance::peek::PeekMode;
use files_model::Input;

use crate::state::Browse;

/// Quick Look, while it is up. The card arrives from the peek worker a moment after the row is
/// chosen; until then, and for a preview that is of the row left, the name stands alone.
#[component]
pub(super) fn QuickLook(browse: Browse) -> Element {
    let shown = browse.state.read().quick_item();
    let Some(item) = shown else {
        return rsx! {};
    };
    let name = item.name.display().into_owned();
    let card = browse
        .looked
        .read()
        .clone()
        .filter(|(of, _)| *of == item)
        .map(|(_, card)| card);
    let body = match card {
        Some(peeked) => rsx! {
            Card { peeked }
        },
        None => rsx! {
            Label { text: name.clone(), style: LabelStyle::Title }
        },
    };
    rsx! {
        Peek {
            mode: PeekMode::Center,
            label: name,
            onclose: move |()| browse.send(Input::CloseQuickLook),
            Column { align: ColumnAlign::Center, extent: ColumnExtent::Fill, {body} }
        }
    }
}
