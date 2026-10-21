//! The folder pane: the toolbar over what the folder holds.

use dioxus::prelude::*;
use ds::components::chrome::column::model::{ColumnExtent, ColumnGap};
use ds::components::chrome::column::view::Column;
use ds::components::overlays::empty_state::EmptyForm;
use ds::prelude::{
    EmptyState, Operation, PendingToken, Progress as Share, ProgressIndicator, ProgressStyle, Shown,
};
use ds::style::tokens::control_size::ControlSize;
use files_core::Location;
use files_model::{Input, ListFailure, Listing};

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
        Listing::Loaded(_) | Listing::Loading => Body::Table,
        // A listing files-model adds later is drawn as a table until this names it.
        _ => Body::Table,
    }
}

/// What an empty place says.
fn empty_words(location: &Location) -> &'static str {
    match location {
        Location::Trash => "Trash is empty",
        Location::Tag(_) => "No files have this tag.",
        Location::Folder(_) | _ => "This folder is empty.",
    }
}

/// Whether the place is still being searched, so its progress shows over the table.
fn walking(browse: Browse) -> bool {
    let state = browse.state.read();
    matches!(state.location(), Location::Tag(_))
        && matches!(state.listing(), Listing::Loading | Listing::Searching(_))
}

/// A bar that turns while a tag search walks the home folder. It starts with the component, which
/// lives exactly as long as the walk.
#[component]
fn Walking() -> Element {
    let running = use_hook(|| Operation::Running(PendingToken::start()));
    rsx! {
        ProgressIndicator {
            style: ProgressStyle::Bar,
            progress: Share::Unknown(running),
            size: ControlSize::Small,
        }
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
            if walking(browse) {
                Walking {}
            }
            {body}
        }
    }
}
