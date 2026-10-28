//! The folder pane: the toolbar over what the folder holds.

use dioxus::prelude::*;
use ds::components::chrome::column::model::ColumnExtent;
use ds::components::chrome::column::view::Column;
use ds::components::overlays::empty_state::EmptyForm;
use ds::prelude::{EmptyState, Shown};
use files_core::{CloudDrive, DriveState, Location, ViewMode};
use files_model::{Input, ListFailure, Listing};

use super::cloud::{DriveStatus, DriveTrouble};
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
    /// The folder is in a cloud drive that cannot show its files: it says why.
    Trouble,
}

/// The drive the folder is in when it has nothing to show because the drive needs signing in or
/// has gone wrong.
fn trouble_of(browse: Browse) -> Option<CloudDrive> {
    let state = browse.state.read();
    let drive = state.drive_here()?;
    let blocked = matches!(drive.state, DriveState::NeedsSignIn | DriveState::Error);
    let nothing = state.rows().is_empty();
    (blocked && nothing).then(|| drive.clone())
}

/// The drive the folder is in, and how many rows it shows, for the status line.
fn status_of(browse: Browse) -> Option<(CloudDrive, usize)> {
    let state = browse.state.read();
    let drive = state.drive_here()?;
    Some((drive.clone(), state.rows().len()))
}

/// What the browser's listing means for the pane.
fn body_of(browse: Browse) -> Body {
    if trouble_of(browse).is_some() {
        return Body::Trouble;
    }
    let state = browse.state.read();
    match state.listing() {
        Listing::Failed(failure) => Body::Failed(*failure),
        Listing::Loaded(_) if state.rows().is_empty() => Body::Empty,
        // A listing files-model adds later is drawn as the view is until this names it.
        Listing::Loaded(_) | Listing::Loading | _ => match state.prefs().view {
            ViewMode::Icons => Body::Icons,
            ViewMode::List | _ => Body::Table,
        },
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
        Body::Trouble => match trouble_of(browse) {
            Some(drive) => rsx! { DriveTrouble { browse, drive } },
            None => rsx! {},
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
        Column { gap: None, extent: ColumnExtent::Fill,
            FolderBar { browse, sidebar }
            TabStrip { browse }
            {body}
            if let Some((drive, items)) = status_of(browse) {
                DriveStatus { drive, items }
            }
        }
    }
}
