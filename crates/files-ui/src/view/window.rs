//! The window: the sidebar pane and the folder pane in a split view, with the keys.

use std::rc::Rc;

use dioxus::prelude::*;
use ds::components::chrome::split_view::model::{Collapsing, PaneSize, PaneSpec, SplitPane};
use ds::components::chrome::split_view::view::SplitView;
use ds::file_drop::drag::FileDrop;
use ds::file_drop::hook::use_file_drop;
use ds::focus::soon::focus_soon;
use ds::prelude::{ActionTaken, Px, Shown};
use ds::window::host::use_window_title;

use super::busy::Busy;
use super::chooser::AppChooser;
use super::goto::GoToSheet;
use super::info::InfoSheet;
use super::menu::{ContextAt, ContextMenu, point_of};
use super::pane::FolderPane;
use super::preview::PreviewColumn;
use super::questions::Questions;
use super::quick_look::QuickLook;
use super::row::dropped;
use super::sidebar::PlacesPane;
use crate::keys::{pressed, use_registered};
use crate::launch::Launch;
use crate::menu::Over;
use crate::places::title_words;
use crate::state::use_browse;

/// The sidebar pane: 232 wide, 212 to 320, and it folds away past half of its least (mailo's).
const SIDEBAR: PaneSpec = PaneSpec {
    preferred: PaneSize::Fixed(Px(232.0)),
    min: PaneSize::Fixed(Px(212.0)),
    max: PaneSize::Fixed(Px(320.0)),
    collapsing: Collapsing::Snaps,
};

/// The window's content, under `Ds`.
#[component]
pub(super) fn Window() -> Element {
    let launch = use_hook(consume_context::<Launch>);
    let browse = use_browse(&launch);
    let keys = use_registered();
    let mut sidebar = use_signal(|| Shown::Visible);
    let mut menu = use_signal(|| None::<ContextAt>);
    // Files dropped from outside on the window (not on a folder row) land in the folder shown.
    let drop = use_file_drop(move |files: FileDrop| browse.send(dropped(files, None)));
    let title = title_words(browse.state.read().location());
    use_window_title(title);
    // The key box, kept so the keyboard can be put back on it: a tab coming to the front draws
    // its own table, and the control that had the keyboard (in the old tab, or a menu that just
    // closed) is gone, which would leave no key a place to land.
    let mut key_box = use_signal(|| None::<Rc<MountedData>>);
    let front = browse.tabs.read().front_id();
    use_effect(use_reactive!(|front| {
        let _ = front;
        if let Some(element) = key_box.peek().clone() {
            focus_soon(element);
        }
    }));
    let shown = sidebar();
    let places = launch
        .places
        .clone()
        .with_sidebar(browse.preferences.read().sidebar.clone());
    let side_body = match shown {
        Shown::Visible => rsx! {
            PlacesPane { browse, places }
        },
        Shown::Hidden => rsx! {},
    };
    rsx! {
        // The box that hears the keys: a press in the table or the sidebar bubbles up to it, and
        // it takes the keyboard as the window opens so the first press has somewhere to land.
        div {
            class: "files-app",
            tabindex: "0",
            "data-drop": drop.drop_attr(),
            onmounted: move |event: MountedEvent| {
                drop.mounted(event.clone());
                key_box.set(Some(event.data()));
                focus_soon(event.data());
            },
            // A press on a row asks for its own menu and keeps the press from reaching here.
            oncontextmenu: move |event: MouseEvent| {
                event.prevent_default();
                menu.set(Some(ContextAt {
                    at: point_of(&event),
                    over: Over::Background,
                }));
            },
            onkeydown: move |event: KeyboardEvent| {
                if pressed(keys, browse, sidebar, &event) == ActionTaken::Yes {
                    event.prevent_default();
                    event.stop_propagation();
                }
            },
            SplitView {
                label: "Files",
                panes: vec![SplitPane::new(SIDEBAR, side_body).shown(shown)],
                on_shown: move |(_, now): (usize, Shown)| sidebar.set(now),
                // The folder keeps its place in the tree whether the preview pane is up or not, so
                // putting the pane away leaves the scroll where it was.
                div { class: "files-body",
                    div { class: "files-main",
                        FolderPane { browse, sidebar, menu, zone: launch.zone }
                    }
                    if browse.state.read().pane_shown() {
                        div { class: "files-preview",
                            PreviewColumn { browse, zone: launch.zone }
                        }
                    }
                }
            }
            ContextMenu { browse, menu }
            Questions { browse }
            QuickLook { browse }
            InfoSheet { browse, zone: launch.zone }
            GoToSheet { browse }
            AppChooser { browse }
            Busy { browse }
        }
    }
}
