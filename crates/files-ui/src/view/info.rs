//! Get Info: a quire `Sheet` of a `Form`, the item's icon and name over its facts as `FactList`s.
//! Name and extension are read-only for now.

use dioxus::prelude::*;
use ds::components::content::label::LabelStyle;
use ds::components::controls::button_model::Answers;
use ds::prelude::{
    Button, Column, ColumnAlign, Fact, FactList, FileIcon, FileIconChoice, Form, FormSection,
    IconSize, Label, Sheet,
};
use ds::style::icon::render::IconPx;
use files_core::{ByteSize, EntryKind, Info, ItemRef};
use files_model::{Browser, InfoView, Input};

use crate::edge::Zone;
use crate::icons::Icons;
use crate::state::Browse;
use crate::words::{InfoRow, drive_rows, info_failure_words, info_rows, item_rows, name_rows};

/// The facts as the rows of a `FactList`.
fn facts(rows: Vec<InfoRow>) -> Vec<Fact> {
    rows.into_iter()
        .map(|row| Fact::new(row.label, row.value))
        .collect()
}

/// The icon of an item whose facts are not read yet: the one its name gives a file.
fn unread(icons: &Icons, item: &ItemRef) -> FileIconChoice {
    icons.choice_of(&item.folder, &item.name, EntryKind::File(ByteSize(0)))
}

/// The item's icon over its name.
fn heading(choice: FileIconChoice, name: String) -> Element {
    rsx! {
        Column { align: ColumnAlign::Center,             FileIcon { choice, size: IconSize::Px(IconPx(64)) }
            Label { text: name, style: LabelStyle::Title }
        }
    }
}

/// The rows a cloud drive adds: its own for the drive's folder, the item's for an item of the
/// folder shown that is in a drive.
fn cloud_rows(state: &Browser, info: &Info) -> Vec<InfoRow> {
    let own = state.drive_at(&info.item().as_folder());
    if let Some(drive) = own {
        return drive_rows(drive);
    }
    if state.folder() != Some(&info.folder) {
        return Vec::new();
    }
    state
        .cloud_of(&info.name)
        .map(item_rows)
        .unwrap_or_default()
}

/// What the sheet holds for each state of the panel.
fn body_of(
    view: &InfoView,
    zone: Zone,
    icons: &Icons,
    cloud: Vec<InfoRow>,
) -> Option<(String, Element)> {
    match view {
        InfoView::Closed => None,
        InfoView::Loading(item) => {
            let name = item.name.display().into_owned();
            let choice = unread(icons, item);
            Some((name.clone(), heading(choice, name)))
        }
        InfoView::Failed(item, failure) => {
            let name = item.name.display().into_owned();
            let said = info_failure_words(*failure);
            let choice = unread(icons, item);
            let body = rsx! {
                {heading(choice, name.clone())}
                Label { text: said }
            };
            Some((name, body))
        }
        InfoView::Shown(info) => Some((
            info.name.display().into_owned(),
            shown(info, zone, icons, cloud),
        )),
        // A state files-model adds later draws nothing until this names it.
        _ => None,
    }
}

/// The facts of `info`: its heading, then its rows and its name.
fn shown(info: &Info, zone: Zone, icons: &Icons, cloud: Vec<InfoRow>) -> Element {
    let rows = info_rows(info, |at| zone.words(at));
    let names = name_rows(info);
    let choice = icons.choice_of(&info.folder, &info.name, info.kind.clone());
    rsx! {
        {heading(choice, info.name.display().into_owned())}
        FormSection { FactList { facts: facts(rows) } }
        if !cloud.is_empty() {
            FormSection { title: Some("Cloud".to_owned()), FactList { facts: facts(cloud) } }
        }
        FormSection { title: Some("Name & Extension".to_owned()), FactList { facts: facts(names) } }
    }
}

/// Get Info, while it is up.
#[component]
pub(super) fn InfoSheet(browse: Browse, zone: Zone) -> Element {
    let view = browse.state.read().info().clone();
    let icons = browse.icons.read().clone();
    let cloud = {
        let state = browse.state.read();
        match &view {
            InfoView::Shown(info) => cloud_rows(&state, info),
            InfoView::Closed | InfoView::Loading(_) | InfoView::Failed(..) | _ => Vec::new(),
        }
    };
    let Some((name, body)) = body_of(&view, zone, &icons, cloud) else {
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
