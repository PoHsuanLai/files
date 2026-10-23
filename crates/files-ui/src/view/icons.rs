//! The icon view: quire's `CollectionView` over the browser's rows, a grid of tiles.

use dioxus::prelude::*;
use ds::base::selection::Pick;
use ds::components::lists::collection_view::{CollectionItem, CollectionView, Thumbnail};
use ds::prelude::FileIconChoice;
use files_core::{Entry, FileName, ItemRef};
use files_model::{Browser, Input, Rename};

use super::cloud::CloudBadge;
use super::rename::RenameCell;
use super::wanted::Wanted;
use crate::cloud::badge_of;
use crate::rows::{click_of, cursor_of, selection_of};
use crate::state::Browse;
use crate::thumbs::{ThumbKey, Thumbed, Thumbs};

/// Where `entry` is, in the folder shown; `None` in the Trash.
fn item_for(state: &Browser, entry: &Entry) -> Option<ItemRef> {
    let folder = state.folder()?;
    Some(ItemRef::new(folder.clone(), entry.name.clone()))
}

/// What a tile draws for an icon: the file's icon, drawn whole at the tile's size.
fn thumbnail_of(choice: FileIconChoice) -> Thumbnail {
    Thumbnail::FileIcon(choice)
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
    let icon = thumbnail_of(browse.icons.read().choice_for(state.folder(), entry));
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
    let badge = badge_of(state.cloud_of(&entry.name));
    let item = match (wish, state.drive_here().is_some()) {
        (None, false) => item,
        (wish, _) => item.badged(rsx! {
            if let Some(wish) = wish {
                {wish}
            }
            CloudBadge { badge }
        }),
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
