//! The folder pane: the toolbar over what the folder holds.

use dioxus::prelude::*;
use ds::components::chrome::column::model::{ColumnExtent, ColumnGap};
use ds::components::chrome::column::view::Column;
use ds::components::overlays::empty_state::EmptyForm;
use ds::prelude::{EmptyState, Shown};
use files_model::{Input, ListFailure, Listing};

use super::table::FolderTable;
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

/// The folder pane.
#[component]
pub(super) fn FolderPane(browse: Browse, sidebar: Signal<Shown>, zone: Zone) -> Element {
    let body = match body_of(browse) {
        Body::Table => rsx! { FolderTable { browse, zone } },
        Body::Empty => rsx! {
            EmptyState { form: EmptyForm::Empty, title: "This folder is empty." }
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
            {body}
        }
    }
}
