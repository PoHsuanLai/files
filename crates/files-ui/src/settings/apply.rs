//! What the settings file says, as the window and the program take it.

use std::path::PathBuf;

use files_core::Behaviour;

use super::words::{NewWindowsShow, Row};
use super::{FilesDoc, SidebarSettings};
use crate::places::{PlaceKind, Sidebar};
use crate::preferences::Preferences;

/// Where a new window opens, before the program has found the user folders.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NewWindow {
    /// The home folder.
    Home,
    /// The Desktop folder.
    Desktop,
    /// The Documents folder.
    Documents,
    /// The Downloads folder.
    Downloads,
    /// This folder, as the person wrote it (`~` is the program's to expand).
    Folder(PathBuf),
}

impl FilesDoc {
    /// Where a new window opens. A chosen folder with no path written is the home folder.
    pub fn new_window(&self) -> NewWindow {
        let general = &self.general;
        match general.new_windows_show {
            NewWindowsShow::Home => NewWindow::Home,
            NewWindowsShow::Desktop => NewWindow::Desktop,
            NewWindowsShow::Documents => NewWindow::Documents,
            NewWindowsShow::Downloads => NewWindow::Downloads,
            NewWindowsShow::Folder => match general.new_windows_folder.trim() {
                "" => NewWindow::Home,
                path => NewWindow::Folder(PathBuf::from(path)),
            },
        }
    }

    /// What the window follows while it is open.
    pub fn preferences(&self) -> Preferences {
        let advanced = &self.advanced;
        Preferences {
            behaviour: Behaviour {
                folders_first: advanced.folders_on_top.is_on(),
                ask_before_emptying_trash: advanced.warn_emptying_trash.is_on(),
                ask_before_changing_extension: advanced.warn_changing_extension.is_on(),
            },
            sidebar: sidebar_of(&self.sidebar),
        }
    }
}

/// The sidebar the rows chosen make.
fn sidebar_of(rows: &SidebarSettings) -> Sidebar {
    let places = [
        (rows.home, PlaceKind::Home),
        (rows.desktop, PlaceKind::Desktop),
        (rows.documents, PlaceKind::Documents),
        (rows.downloads, PlaceKind::Downloads),
        (rows.pictures, PlaceKind::Pictures),
        (rows.music, PlaceKind::Music),
        (rows.videos, PlaceKind::Videos),
        (rows.computer, PlaceKind::Computer),
    ];
    places
        .into_iter()
        .filter(|(row, _)| *row == Row::Hide)
        .fold(Sidebar::default(), |sidebar, (_, kind)| {
            sidebar.leaving_out(kind)
        })
        .with_disks(rows.disks.is_shown())
}
