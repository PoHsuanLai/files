//! The window: the sidebar pane and the folder pane in a split view, with the keys.

use dioxus::prelude::*;
use ds::components::chrome::split_view::model::{Collapsing, PaneSize, PaneSpec, SplitPane};
use ds::components::chrome::split_view::view::SplitView;
use ds::focus::soon::focus_soon;
use ds::prelude::{ActionTaken, Px, Shown};
use ds::window::host::use_window_title;

use super::pane::FolderPane;
use super::sidebar::PlacesPane;
use crate::keys::{pressed, use_registered};
use crate::launch::Launch;
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
    let title = title_words(browse.state.read().location());
    use_window_title(title);
    let shown = sidebar();
    let side_body = match shown {
        Shown::Visible => rsx! {
            PlacesPane { browse, places: launch.places.clone() }
        },
        Shown::Hidden => rsx! {},
    };
    rsx! {
        // The box that hears the keys: a press in the table or the sidebar bubbles up to it, and
        // it takes the keyboard as the window opens so the first press has somewhere to land.
        div {
            class: "files-app",
            tabindex: "0",
            onmounted: move |event: MountedEvent| focus_soon(event.data()),
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
                FolderPane { browse, sidebar, zone: launch.zone }
            }
        }
    }
}
