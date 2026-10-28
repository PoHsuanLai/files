//! What every cell of a row shares: a right click asks for the context menu, and (for a folder)
//! files dropped on it land inside it.

use dioxus::prelude::*;
use ds::file_drop::drag::FileDrop;
use ds::file_drop::hook::use_file_drop;
use files_core::{FileName, ItemRef};
use files_model::{DropHow, Input, Modifier};

use super::menu::{ContextAt, point_of};
use crate::menu::Over;
use crate::state::Browse;

/// The input for files let go from outside the window, on the folder row `onto` or, with `None`,
/// on the folder shown. quire offers the drop as a copy to the source, so it is a copy.
pub(super) fn dropped(files: FileDrop, onto: Option<FileName>) -> Input {
    let sources = files
        .paths
        .iter()
        .filter_map(|path| ItemRef::from_path(path).ok())
        .collect();
    Input::Drop {
        sources,
        onto,
        how: DropHow::Copy,
    }
}

/// A right click on the row `name`: the row is selected first if it is not (as Finder does), then
/// the menu is asked for.
fn asked(browse: Browse, mut menu: Signal<Option<ContextAt>>, name: &FileName, event: &MouseEvent) {
    // A name being typed has its own menu, which is the field's.
    if browse.renaming() {
        return;
    }
    event.prevent_default();
    event.stop_propagation();
    let selected = browse.state.read().selection().contains(name);
    if !selected {
        browse.send(Input::Click(name.clone(), Modifier::None));
    }
    menu.set(Some(ContextAt {
        at: point_of(event),
        over: Over::Row,
    }));
}

/// One cell's content in its shell.
#[component]
pub(super) fn RowCell(
    browse: Browse,
    menu: Signal<Option<ContextAt>>,
    name: FileName,
    drops: bool,
    content: Element,
) -> Element {
    // A folder row takes a drop inside it; any other row lands it in the folder shown.
    if drops {
        rsx! {
            FolderCell { browse, menu, name, content }
        }
    } else {
        rsx! {
            PlainCell { browse, menu, name, content }
        }
    }
}

#[component]
fn PlainCell(
    browse: Browse,
    menu: Signal<Option<ContextAt>>,
    name: FileName,
    content: Element,
) -> Element {
    rsx! {
        div {
            oncontextmenu: move |event: MouseEvent| asked(browse, menu, &name, &event),
            {content}
        }
    }
}

#[component]
fn FolderCell(
    browse: Browse,
    menu: Signal<Option<ContextAt>>,
    name: FileName,
    content: Element,
) -> Element {
    let target = name.clone();
    let drop = use_file_drop(move |files: FileDrop| {
        browse.send(dropped(files, Some(target.clone())));
    });
    rsx! {
        div {
            "data-drop": drop.drop_attr(),
            onmounted: move |event: MountedEvent| drop.mounted(event),
            oncontextmenu: move |event: MouseEvent| asked(browse, menu, &name, &event),
            {content}
        }
    }
}
