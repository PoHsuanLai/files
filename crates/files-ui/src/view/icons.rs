//! The icon view: quire's `CollectionView` over the browser's rows, a grid of tiles.

use dioxus::prelude::*;
use ds::base::selection::Pick;
use ds::components::lists::collection_view::{CollectionItem, CollectionView, Thumbnail};
use ds::prelude::IconSource;
use files_core::{Entry, FileName, ItemRef};
use files_model::{Browser, Input, Rename};

use super::rename::RenameCell;
use super::table::icon_of;
use super::wanted::Wanted;
use crate::rows::{click_of, cursor_of, selection_of};
use crate::state::Browse;
use crate::thumbs::{ThumbKey, Thumbed, Thumbs};

/// Where `entry` is: where a search found it, else in the folder shown; `None` in the Trash.
fn item_for(state: &Browser, entry: &Entry) -> Option<ItemRef> {
    match (&entry.found, state.folder()) {
        (Some(item), _) => Some(item.clone()),
        (None, Some(folder)) => Some(ItemRef::new(folder.clone(), entry.name.clone())),
        (None, None) => None,
    }
}

/// The picture of the tile of `entry`: its thumbnail once it has one, else its icon; and, for a
/// file that may have a thumbnail and has not been given its answer, the wish for one, which
/// lasts as long as the tile is drawn.
fn picture_of(
    browse: Browse,
    state: &Browser,
    shelf: &Thumbs,
    entry: &Entry,
) -> (Thumbnail, Option<Element>) {
    let icon = Thumbnail::Icon(IconSource::from(icon_of(entry)));
    let Some(key) = item_for(state, entry).and_then(|item| ThumbKey::of(item, entry)) else {
        return (icon, None);
    };
    match shelf.get(&key) {
        Some(Thumbed::Picture(source)) => (Thumbnail::Image(source.clone()), None),
        Some(Thumbed::Icon) => (icon, None),
        None => (icon, Some(rsx! { Wanted { browse, thumb: key } })),
    }
}

/// The tile of `entry`: its thumbnail or glyph over its name, or its name field while it is being
/// renamed.
fn item_of(
    browse: Browse,
    state: &Browser,
    shelf: &Thumbs,
    entry: &Entry,
) -> CollectionItem<FileName> {
    let (thumbnail, wish) = picture_of(browse, state, shelf, entry);
    let item = CollectionItem::new(
        entry.name.clone(),
        entry.shown_name().display().into_owned(),
        thumbnail,
    );
    let item = match wish {
        Some(wish) => item.badged(wish),
        None => item,
    };
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
        let shelf = browse.thumbs.read();
        let items: Vec<_> = state
            .rows()
            .iter()
            .map(|entry| item_of(browse, &state, &shelf, entry))
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
