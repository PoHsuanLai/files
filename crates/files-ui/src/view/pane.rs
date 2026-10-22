//! The folder pane: the toolbar over what the folder holds.

use dioxus::prelude::*;
use ds::components::chrome::column::model::{ColumnExtent, ColumnGap};
use ds::components::chrome::column::view::Column;
use ds::components::overlays::empty_state::EmptyForm;
use ds::prelude::{EmptyState, Shown};
use files_core::{Location, ViewMode};
use files_model::{Input, ListFailure, Listing};

use super::icons::FolderIcons;
use super::menu::ContextAt;
use super::table::FolderTable;
use super::tabs::TabStrip;
use super::toolbar::FolderBar;
use crate::edge::Zone;
use crate::state::Browse;

/// What the pane holds under the toolbar.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Body {
    /// The table: rows, or none yet while the folder is being read.
    Table,
    /// The grid of tiles, for the same rows.
    Icons,
    /// The folder has nothing to show.
    Empty,
    /// The folder could not be read.
    Failed(ListFailure),
}

/// What the browser's listing means for the pane.
fn body_of(browse: Browse) -> Body {
    let state = browse.state.read();
    match state.listing() {
        Listing::Failed(failure) => Body::Failed(*failure),
        Listing::Loaded(_) if state.rows().is_empty() => Body::Empty,
        // A listing files-model adds later is drawn as the view is until this names it.
        Listing::Loaded(_) | Listing::Loading | _ => {
            match state.prefs().view {
                ViewMode::Icons => Body::Icons,
                ViewMode::List | _ => Body::Table,
            }
        }
    }
}

/// What an empty place says.
fn empty_words(location: &Location) -> &'static str {
    match location {
        Location::Trash => "Trash is empty",
        Location::Folder(_) | _ => "This folder is empty.",
    }
}

/// The folder pane.
#[component]
pub(super) fn FolderPane(
    browse: Browse,
    sidebar: Signal<Shown>,
    menu: Signal<Option<ContextAt>>,
    zone: Zone,
) -> Element {
    let body = match body_of(browse) {
        Body::Table => rsx! { FolderTable { browse, menu, zone } },
        Body::Icons => rsx! { FolderIcons { browse } },
        Body::Empty => rsx! {
            EmptyState { form: EmptyForm::Empty, title: empty_words(browse.state.read().location()) }
        },
        Body::Failed(failure) => rsx! {
            EmptyState {
                form: EmptyForm::Failure,
                title: failure.to_string(),
                onretry: move |()| browse.send(Input::Reload),
            }
        },
    };
    rsx! {
        Column { gap: ColumnGap::None, extent: ColumnExtent::Fill,
            FolderBar { browse, sidebar }
            TabStrip { browse }
            {body}
        }
    }
}
