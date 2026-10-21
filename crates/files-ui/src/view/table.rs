//! The list view: quire's `VirtualTable` over the browser's rows.

use dioxus::prelude::*;
use ds::base::selection::Pick;
use ds::components::controls::scroller::handle::use_scroller;
use ds::components::lists::table::model::{CellAlign, Sort as TableSort, TableColumn};
use ds::components::lists::virtual_table::VirtualTable;
use ds::prelude::{Icon, IconSource, IconView, Label, Px, TableLook};
use files_core::{Entry, Sort};
use files_model::{Browser, Input, Opens, Rename, opens};

use super::rename::RenameCell;
use crate::edge::Zone;
use crate::rows::{Column, cursor_of, direction_of, modifier_of, order_of, selection_of};
use crate::state::Browse;
use crate::words::{entry_size_words, kind_words};

/// A column of the table: its header and width.
fn column(id: Column) -> TableColumn<Column> {
    let width = match id {
        Column::Name => Px(280.0),
        Column::Modified => Px(160.0),
        Column::Size => Px(96.0),
        Column::Kind => Px(140.0),
    };
    let base = TableColumn::new(id, id.title(), width);
    match id {
        Column::Size => base.aligned(CellAlign::Trailing),
        Column::Name | Column::Modified | Column::Kind => base,
    }
}

/// The glyph before a name. quire has no per-type file glyphs yet, so a file is a file.
fn icon_of(entry: &Entry) -> Icon {
    match opens(entry) {
        Opens::Folder => Icon::Folder,
        Opens::File | Opens::Nothing => Icon::File,
    }
}

/// What the Name column shows for `entry`: its name field while it is being renamed, else its
/// glyph and name.
fn name_cell(browse: Browse, state: &Browser, entry: &Entry) -> Element {
    if let Rename::Editing {
        name,
        draft,
        problem,
    } = state.rename()
        && *name == entry.name
    {
        return rsx! {
            RenameCell { browse, draft: draft.clone(), problem: *problem }
        };
    }
    rsx! {
        IconView { source: IconSource::from(icon_of(entry)) }
        Label { text: entry.name.display().into_owned() }
    }
}

/// `content` that opens the selection when it is double-clicked: the table reports presses and
/// keys but not a double click.
fn openable(browse: Browse, content: Element) -> Element {
    rsx! {
        div { ondoubleclick: move |_| browse.send(Input::Open), {content} }
    }
}

/// The folder's rows as a striped table (Finder's list view): Name pinned, the others scrolling
/// sideways under it; a header press sorts, a click selects, Shift extends, Command toggles.
#[component]
pub(super) fn FolderTable(browse: Browse, zone: Zone) -> Element {
    let scroller = use_scroller();
    let (count, sort, cursor, selection) = {
        let state = browse.state.read();
        let prefs = state.prefs();
        let sort = TableSort {
            column: Column::of_key(prefs.sort.key),
            direction: direction_of(prefs.sort.order),
        };
        (
            state.rows().len(),
            sort,
            cursor_of(&state),
            selection_of(&state),
        )
    };
    let cell = use_callback(move |(row, at): (usize, usize)| -> Element {
        let state = browse.state.read();
        let (Some(entry), Some(id)) = (state.rows().get(row), Column::ALL.get(at).copied()) else {
            return rsx! {};
        };
        let content = match id {
            Column::Name => name_cell(browse, &state, entry),
            Column::Modified => {
                let words = zone.words(entry.modified);
                rsx! { "{words}" }
            }
            Column::Size => {
                let words = entry_size_words(entry);
                rsx! { "{words}" }
            }
            Column::Kind => {
                let words = kind_words(entry);
                rsx! { "{words}" }
            }
        };
        openable(browse, content)
    });
    rsx! {
        VirtualTable::<Column> {
            label: "Folder".to_owned(),
            columns: Column::ALL.map(column).to_vec(),
            rows: count,
            cell,
            scroller,
            sort: Some(sort),
            on_sort: move |sort: TableSort<Column>| {
                let sort = Sort {
                    key: sort.column.key(),
                    order: order_of(sort.direction),
                };
                browse.send(Input::SetSort(sort));
            },
            cursor,
            selection: Some(selection),
            onreach: move |pick: Pick| {
                let name = browse
                    .state
                    .read()
                    .rows()
                    .get(pick.row)
                    .map(|row| row.name.clone());
                if let Some(name) = name {
                    browse.send(Input::Click(name, modifier_of(pick.how)));
                }
            },
            pinned: 1,
            look: TableLook::Striped,
        }
    }
}
