//! The list view: quire's `VirtualTable` over the browser's rows.

use dioxus::prelude::*;
use ds::base::selection::Pick;
use ds::components::controls::scroller::handle::use_scroller;
use ds::components::lists::table::model::{CellAlign, Sort as TableSort, Sorting, TableColumn};
use ds::components::lists::virtual_table::VirtualTable;
use ds::prelude::{FileIcon, Label, Px, TableLook};
use files_core::{Entry, Location, Sort, SortKey};
use files_model::{Browser, Input, Opens, Rename, opens};

use super::cloud::CloudBadge;
use super::menu::ContextAt;
use super::rename::RenameCell;
use super::row::RowCell;
use crate::cloud::badge_of;
use crate::edge::Zone;
use crate::rows::{Column, click_of, cursor_of, direction_of, order_of, selection_of};
use crate::state::Browse;
use crate::words::{NONE, entry_size_words, kind_words};

/// A column of the table: its header and width.
fn column(id: Column, location: &Location) -> TableColumn<Column> {
    let width = match id {
        Column::WhereFrom => Px(200.0),
        Column::Sort(SortKey::Modified) => Px(160.0),
        Column::Sort(SortKey::Size) => Px(96.0),
        Column::Sort(SortKey::Kind) => Px(140.0),
        // The name, and a key files-core adds later.
        Column::Sort(_) => Px(280.0),
    };
    let base = TableColumn::new(id, id.title_in(location), width);
    match id {
        Column::Sort(SortKey::Size) => base.aligned(CellAlign::Trailing),
        Column::WhereFrom => base.sorting(Sorting::Fixed),
        Column::Sort(_) => base,
    }
}

/// What the Name column shows for `entry`: its name field while it is being renamed, else its
/// icon and name.
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
    let choice = browse.icons.read().choice_for(state.folder(), entry);
    let badge = badge_of(state.cloud_of(&entry.name));
    rsx! {
        FileIcon { choice }
        Label { text: entry.shown_name().display().into_owned() }
        if state.drive_here().is_some() {
            CloudBadge { badge }
        }
    }
}

/// Where a trashed item was before it was thrown away; a dash for anything else.
fn origin_words(entry: &Entry) -> String {
    match &entry.trashed {
        Some(item) => item.original.folder.as_path().display().to_string(),
        None => NONE.to_owned(),
    }
}

/// `content` in the shell every cell of the row shares (context menu, drops).
fn shell(
    browse: Browse,
    menu: Signal<Option<ContextAt>>,
    entry: &Entry,
    content: Element,
) -> Element {
    let drops = opens(entry) == Opens::Folder;
    let name = entry.name.clone();
    rsx! {
        RowCell { browse, menu, name, drops, content }
    }
}

/// The folder's rows as a striped table (Finder's list view): Name pinned, the others scrolling
/// sideways under it; a header press sorts, a click selects, Shift extends, Command toggles.
#[component]
pub(super) fn FolderTable(browse: Browse, menu: Signal<Option<ContextAt>>, zone: Zone) -> Element {
    let scroller = use_scroller();
    let (count, sort, cursor, selection, location) = {
        let state = browse.state.read();
        let prefs = state.prefs();
        let sort = TableSort {
            column: Column::Sort(prefs.sort.key),
            direction: direction_of(prefs.sort.order),
        };
        (
            state.rows().len(),
            sort,
            cursor_of(&state),
            selection_of(&state),
            state.location().clone(),
        )
    };
    let cell = use_callback(move |(row, at): (usize, usize)| -> Element {
        let state = browse.state.read();
        let columns = Column::of_location(state.location());
        let (Some(entry), Some(id)) = (state.rows().get(row), columns.get(at).copied()) else {
            return rsx! {};
        };
        let content = match id {
            Column::Sort(SortKey::Modified) => {
                let words = zone.words(entry.modified);
                rsx! { "{words}" }
            }
            Column::Sort(SortKey::Size) => {
                let words = entry_size_words(entry);
                rsx! { "{words}" }
            }
            Column::Sort(SortKey::Kind) => {
                let words = kind_words(entry);
                rsx! { "{words}" }
            }
            Column::WhereFrom => {
                let words = origin_words(entry);
                rsx! { "{words}" }
            }
            // The name, and a key files-core adds later.
            Column::Sort(_) => name_cell(browse, &state, entry),
        };
        shell(browse, menu, entry, content)
    });
    rsx! {
        VirtualTable::<Column> {
            label: "Folder".to_owned(),
            columns: Column::of_location(&location)
                .iter()
                .map(|id| column(*id, &location))
                .collect::<Vec<_>>(),
            rows: count,
            cell,
            scroller,
            sort: Some(sort),
            on_sort: move |sort: TableSort<Column>| {
                if let Some(key) = sort.column.sort_key() {
                    let sort = Sort {
                        key,
                        order: order_of(sort.direction),
                    };
                    browse.send(Input::SetSort(sort));
                }
            },
            cursor,
            selection: Some(selection),
            onreach: move |pick: Pick| {
                let click = click_of(&browse.state.read(), pick);
                if let Some(click) = click {
                    browse.send(click);
                }
            },
            // A double click, or Command+Down on the cursor row: Finder's Open.
            onopen: move |_: usize| browse.send(Input::Open),
            pinned: 1,
            look: TableLook::Striped,
        }
    }
}
