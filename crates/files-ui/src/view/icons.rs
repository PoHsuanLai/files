//! The icon view: quire's `CollectionView` over the browser's rows, a grid of tiles.

use dioxus::prelude::*;
use ds::base::selection::Pick;
use ds::components::lists::collection_view::{CollectionItem, CollectionView, Thumbnail};
use ds::prelude::IconSource;
use files_core::{Entry, FileName};
use files_model::{Browser, Input, Rename};

use super::rename::RenameCell;
use super::table::icon_of;
use crate::rows::{click_of, cursor_of, selection_of};
use crate::state::Browse;

/// The tile of `entry`: its glyph over its name, or its name field while it is being renamed.
fn item_of(browse: Browse, state: &Browser, entry: &Entry) -> CollectionItem<FileName> {
    let item = CollectionItem::new(
        entry.name.clone(),
        entry.shown_name().display().into_owned(),
        Thumbnail::Icon(IconSource::from(icon_of(entry))),
    );
    if let Rename::Editing {
        name,
        draft,
        problem,
    } = state.rename()
        && *name == entry.name
    {
        return item.editing(rsx! {
            RenameCell { browse, draft: draft.clone(), problem: *problem }
        });
    }
    item
}

/// The folder's rows as a grid of tiles (Finder's icon view); a click selects, Shift extends,
/// Command toggles, a double click or Command+Down opens.
#[component]
pub(super) fn FolderIcons(browse: Browse) -> Element {
    let (items, cursor, selection) = {
        let state = browse.state.read();
        let items: Vec<_> = state
            .rows()
            .iter()
            .map(|entry| item_of(browse, &state, entry))
            .collect();
        (items, cursor_of(&state), selection_of(&state))
    };
    rsx! {
        CollectionView::<FileName> {
            label: "Folder".to_owned(),
            items,
            cursor,
            selection: Some(selection),
            onreach: move |pick: Pick| {
                let click = click_of(&browse.state.read(), pick);
                if let Some(click) = click {
                    browse.send(click);
                }
            },
            onopen: move |_: usize| browse.send(Input::Open),
        }
    }
}
