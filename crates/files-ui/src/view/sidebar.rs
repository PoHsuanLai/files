//! The sidebar: Favorites and Locations in quire's source list. Under Locations, between the
//! computer and the Trash, are the disks that are attached, each with its eject button.

use dioxus::prelude::*;
use ds::base::press::Press;
use ds::base::vocab::RowState;
use ds::components::chrome::sidebar::Sidebar;
use ds::components::chrome::sidebar_model::SidebarSection;
use ds::prelude::{Icon, ListItem, Row, RowAction, RowLeading, SectionHeader, Selection};
use files_core::{Eject, Volume, VolumeId, VolumeKind};
use files_model::Input;

use crate::places::{DiskRows, Heading, PlaceKey, PlaceKind, Places, groups, volume_key};
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
        PlaceKind::Trash => Icon::Trash,
    }
}

/// The glyph of a disk. quire has no drive glyph yet, so a disk is a folder, and a disc a speaker's
/// nearest relative, the music note.
fn icon_of_volume(kind: VolumeKind) -> Icon {
    match kind {
        VolumeKind::Optical => Icon::Music,
        VolumeKind::External | VolumeKind::Internal | _ => Icon::Folder,
    }
}

/// What the sidebar's rows hear: a press on a row, and a press on a disk's eject button.
#[derive(Clone, Copy, PartialEq)]
pub(super) struct Hears {
    /// A row was picked.
    pub(super) onselect: EventHandler<PlaceKey>,
    /// A disk's eject button was pressed.
    pub(super) oneject: EventHandler<VolumeId>,
}

/// The sidebar's sections for `places` and the attached `volumes`: a list of headings and rows,
/// `cursor` drawn selected.
pub(super) fn sidebar_rows(
    places: &Places,
    volumes: &[Volume],
    cursor: Option<PlaceKey>,
    hears: Hears,
) -> Vec<SidebarSection<PlaceKey>> {
    let selection_of = move |key: PlaceKey| {
        if cursor == Some(key) {
            Selection::Selected
        } else {
            Selection::Unselected
        }
    };
    let items = groups(places)
        .into_iter()
        .flat_map(|group| {
            let heading = ListItem::heading(
                PlaceKey::Heading(group.heading),
                rsx! { SectionHeader { title: group.heading.label().to_owned() } },
            );
            let mut rows: Vec<ListItem<PlaceKey>> = group
                .rows
                .into_iter()
                .map(|kind| {
                    let key = PlaceKey::Place(kind);
                    let selection = selection_of(key.clone());
                    let onselect = hears.onselect;
                    ListItem::row(
                        key,
                        kind.label(),
                        rsx! { PlaceRow { kind, selection, onselect } },
                    )
                })
                .collect();
            if group.heading == Heading::Locations {
                // The disks go before the last row, the Trash.
                let at = rows.len().saturating_sub(1);
                let listed = places.sidebar().disks() == DiskRows::Listed;
                let disks = volumes.iter().filter(|_| listed).map(|volume| {
                    let key = PlaceKey::Volume(volume.id.clone());
                    let selection = selection_of(key.clone());
                    ListItem::row(
                        key,
                        volume.name.as_str().to_owned(),
                        rsx! { VolumeRow { volume: volume.clone(), selection, hears } },
                    )
                });
                let tail = rows.split_off(at);
                rows.extend(disks);
                rows.extend(tail);
            }
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

/// One disk's row: its name, and an eject button when it can be ejected.
#[component]
fn VolumeRow(volume: Volume, selection: Selection, hears: Hears) -> Element {
    let state = RowState {
        selection,
        ..RowState::default()
    };
    let leading = RowLeading::Icon(icon_of_volume(volume.kind));
    let title = volume.name.as_str().to_owned();
    let key = PlaceKey::Volume(volume.id.clone());
    let onclick = EventHandler::new(move |_: Press| hears.onselect.call(key.clone()));
    match volume.eject {
        Eject::Allowed => {
            let id = volume.id.clone();
            let action = RowAction::new(
                Icon::ChevronUp,
                format!("Eject {}", volume.name),
                EventHandler::new(move |_| hears.oneject.call(id.clone())),
            );
            rsx! {
                Row { leading, title, state, action, onclick }
            }
        }
        Eject::Never | _ => rsx! {
            Row { leading, title, state, onclick }
        },
    }
}

/// The sidebar pane: where the window is among the places, and a press goes there.
#[component]
pub(super) fn PlacesPane(browse: Browse, places: Places) -> Element {
    let volumes = browse.state.read().volumes().to_vec();
    let cursor = {
        let state = browse.state.read();
        places
            .key_of(state.location())
            .or_else(|| volume_key(state.volumes(), state.location()))
    };
    let go = use_callback({
        let places = places.clone();
        move |key: PlaceKey| match key {
            PlaceKey::Place(kind) => {
                if let Some(location) = places.location_of(kind) {
                    browse.send(Input::Go(location));
                }
            }
            PlaceKey::Volume(id) => browse.send(Input::OpenVolume(id)),
            PlaceKey::Heading(Heading::Favorites | Heading::Locations) => {}
        }
    });
    let eject = use_callback(move |id: VolumeId| browse.send(Input::Eject(id)));
    let hears = Hears {
        onselect: go,
        oneject: eject,
    };
    rsx! {
        Sidebar::<PlaceKey> {
            label: "Sidebar",
            sections: sidebar_rows(&places, &volumes, cursor.clone(), hears),
            cursor,
            onselect: go,
        }
    }
}
