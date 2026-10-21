//! The sidebar: Favorites and Locations in quire's source list.

use dioxus::prelude::*;
use ds::base::vocab::RowState;
use ds::components::chrome::sidebar::Sidebar;
use ds::components::chrome::sidebar_model::SidebarSection;
use ds::prelude::{Icon, ListItem, Row, RowLeading, SectionHeader, Selection};
use files_model::Input;

use crate::places::{Heading, PlaceKey, PlaceKind, Places, groups};
use crate::state::Browse;

/// The glyph of a place. quire has no Home, Desktop or Videos glyph yet, so the nearest stand in.
fn icon_of(kind: PlaceKind) -> Icon {
    match kind {
        PlaceKind::Home => Icon::Folder,
        PlaceKind::Desktop => Icon::Monitor,
        PlaceKind::Documents => Icon::File,
        PlaceKind::Downloads => Icon::Download,
        PlaceKind::Pictures => Icon::Image,
        PlaceKind::Music => Icon::Music,
        PlaceKind::Videos => Icon::Play,
        PlaceKind::Computer => Icon::Window,
    }
}

/// The sidebar's sections for `places`: a list of headings and rows, `cursor` drawn selected,
/// `onselect` hearing a press on a row.
pub(super) fn sidebar_rows(
    places: &Places,
    cursor: Option<PlaceKey>,
    onselect: EventHandler<PlaceKey>,
) -> Vec<SidebarSection<PlaceKey>> {
    let items = groups(places)
        .into_iter()
        .flat_map(|group| {
            let heading = ListItem::heading(
                PlaceKey::Heading(group.heading),
                rsx! { SectionHeader { title: group.heading.label().to_owned() } },
            );
            let rows = group.rows.into_iter().map(move |kind| {
                let key = PlaceKey::Place(kind);
                let selection = if cursor == Some(key) {
                    Selection::Selected
                } else {
                    Selection::Unselected
                };
                ListItem::row(
                    key,
                    kind.label(),
                    rsx! { PlaceRow { kind, selection, onselect } },
                )
            });
            std::iter::once(heading).chain(rows)
        })
        .collect();
    vec![SidebarSection::List(items)]
}

/// One place's row.
#[component]
fn PlaceRow(kind: PlaceKind, selection: Selection, onselect: EventHandler<PlaceKey>) -> Element {
    rsx! {
        Row {
            leading: RowLeading::Icon(icon_of(kind)),
            title: kind.label(),
            state: RowState {
                selection,
                ..RowState::default()
            },
            onclick: move |_| onselect.call(PlaceKey::Place(kind)),
        }
    }
}

/// The sidebar pane: where the window is among the places, and a press goes there.
#[component]
pub(super) fn PlacesPane(browse: Browse, places: Places) -> Element {
    let cursor = places.key_of(browse.state.read().location());
    let go = use_callback({
        let places = places.clone();
        move |key: PlaceKey| match key {
            PlaceKey::Place(kind) => {
                if let Some(location) = places.location_of(kind) {
                    browse.send(Input::Go(location));
                }
            }
            PlaceKey::Heading(Heading::Favorites | Heading::Locations) => {}
        }
    });
    rsx! {
        Sidebar::<PlaceKey> {
            label: "Sidebar",
            sections: sidebar_rows(&places, cursor, go),
            cursor,
            onselect: go,
        }
    }
}
