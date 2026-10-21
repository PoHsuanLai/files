//! Get Info: a quire `Sheet` of a `Form`, the item's icon and name over its facts as `FactList`s.
//! Name and extension are read-only for now.

use dioxus::prelude::*;
use ds::components::chrome::column::model::ColumnGap;
use ds::components::content::label::LabelStyle;
use ds::components::controls::button_model::Answers;
use ds::prelude::{
    Button, Column, ColumnAlign, Fact, FactList, Form, FormSection, Icon, IconSize, IconSource,
    IconView, Label, Sheet,
};
use ds::style::icon::family::PlateFamily;
use ds::style::icon::render::IconPx;
use files_core::{EntryKind, Info, LinkTo};
use files_model::{InfoView, Input};

use crate::edge::Zone;
use crate::state::Browse;
use crate::words::{InfoRow, info_failure_words, info_rows, name_rows};

/// The facts as the rows of a `FactList`.
fn facts(rows: Vec<InfoRow>) -> Vec<Fact> {
    rows.into_iter()
        .map(|row| Fact::new(row.label, row.value))
        .collect()
}

/// The glyph on the item's plate.
fn glyph_of(kind: &EntryKind) -> Icon {
    match kind {
        EntryKind::Folder | EntryKind::Link(LinkTo::Folder) => Icon::Folder,
        EntryKind::File(_) | EntryKind::Link(_) | EntryKind::Special(_) => Icon::File,
        // A kind files-core adds later is a file until this names it.
        _ => Icon::File,
    }
}

/// The item's icon over its name.
fn heading(glyph: Icon, name: String) -> Element {
    rsx! {
        Column { align: ColumnAlign::Center, gap: ColumnGap::default(),
            IconView {
                source: IconSource::Glyph(glyph),
                size: IconSize::Px(IconPx(64)),
                plate: Some(PlateFamily::Blue),
            }
            Label { text: name, style: LabelStyle::Title }
        }
    }
}

/// What the sheet holds for each state of the panel.
fn body_of(view: &InfoView, zone: Zone) -> Option<(String, Element)> {
    match view {
        InfoView::Closed => None,
        InfoView::Loading(item) => {
            let name = item.name.display().into_owned();
            Some((name.clone(), heading(Icon::File, name)))
        }
        InfoView::Failed(item, failure) => {
            let name = item.name.display().into_owned();
            let said = info_failure_words(*failure);
            let body = rsx! {
                {heading(Icon::File, name.clone())}
                Label { text: said }
            };
            Some((name, body))
        }
        InfoView::Shown(info) => Some((info.name.display().into_owned(), shown(info, zone))),
        // A state files-model adds later draws nothing until this names it.
        _ => None,
    }
}

/// The facts of `info`: its heading, then its rows and its name.
fn shown(info: &Info, zone: Zone) -> Element {
    let rows = info_rows(info, |at| zone.words(at));
    let names = name_rows(info);
    rsx! {
        {heading(glyph_of(&info.kind), info.name.display().into_owned())}
        FormSection { FactList { facts: facts(rows) } }
        FormSection { title: Some("Name & Extension".to_owned()), FactList { facts: facts(names) } }
    }
}

/// Get Info, while it is up.
#[component]
pub(super) fn InfoSheet(browse: Browse, zone: Zone) -> Element {
    let view = browse.state.read().info().clone();
    let Some((name, body)) = body_of(&view, zone) else {
        return rsx! {};
    };
    let close = EventHandler::new(move |()| browse.send(Input::CloseInfo));
    rsx! {
        Sheet {
            label: format!("{name} Info"),
            onclose: close,
            on_return: Some(close),
            Form {
                {body}
                Column { align: ColumnAlign::End,
                    Button {
                        label: "Done",
                        answers: Answers::Return,
                        onclick: move |_| close.call(()),
                    }
                }
            }
        }
    }
}
