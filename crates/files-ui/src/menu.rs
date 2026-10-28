//! What the context menu offers: pure, so a table can say it.

use std::path::PathBuf;

use files_core::Location;
use files_model::{AppPick, Browser, Input};

use crate::edge::AppChoiceView;
use crate::words;

/// quire's availability for a yes/no.
pub fn enabled(can: bool) -> ds::prelude::Availability {
    if can {
        ds::prelude::Availability::Enabled
    } else {
        ds::prelude::Availability::Disabled
    }
}

/// One command of the context menu.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Command {
    /// Open the selection.
    Open,
    /// Open the selected folder in a new tab.
    OpenInNewTab,
    /// Open the selection with an app picked from a submenu. It is a parent, never picked itself.
    OpenWith,
    /// Open Quick Look on the selection.
    QuickLook,
    /// Show the properties of the selection.
    GetInfo,
    /// Show the properties of the folder shown.
    GetInfoFolder,
    /// Put the full paths of the selection on the clipboard.
    CopyAsPathname,
    /// Move the selection to the Trash.
    MoveToTrash,
    /// Rename the selected item.
    Rename,
    /// Copy the selection.
    Copy,
    /// Paste what was copied into this folder.
    Paste,
    /// Make a new folder.
    NewFolder,
    /// Put the selection back where it came from (in the Trash).
    PutBack,
    /// Erase the selection for good (in the Trash).
    DeleteImmediately,
    /// Erase everything in the Trash for good.
    EmptyTrash,
    /// Download the selection now (in a cloud drive).
    DownloadNow,
    /// Remove the downloaded copy of the selection, keeping it listed (in a cloud drive).
    RemoveDownload,
    /// Keep the selection downloaded; checked while all of it is (in a cloud drive).
    KeepDownloaded,
    /// Save the online documents of the selection as files next to them (in a cloud drive).
    SaveCopy,
    /// Show the preview pane beside the folder.
    ShowPreview,
    /// Put the preview pane away.
    HidePreview,
}

impl Command {
    /// The menu item's words.
    pub fn title(self) -> &'static str {
        words::command_title(self)
    }

    /// What picking it asks of the browser; `None` for a parent of a submenu, which is not picked.
    pub fn input(self) -> Option<Input> {
        Some(match self {
            Command::OpenWith => return None,
            Command::Open => Input::Open,
            Command::OpenInNewTab => Input::OpenInNewTab,
            Command::QuickLook => Input::QuickLook,
            Command::GetInfo => Input::GetInfo,
            Command::GetInfoFolder => Input::GetInfoFolder,
            Command::CopyAsPathname => Input::CopyAsPathname,
            Command::MoveToTrash => Input::MoveToTrash,
            Command::Rename => Input::BeginRename,
            Command::Copy => Input::Copy,
            Command::Paste => Input::Paste,
            Command::NewFolder => Input::NewFolder,
            Command::PutBack => Input::PutBack,
            Command::DeleteImmediately => Input::DeleteImmediately,
            Command::EmptyTrash => Input::EmptyTrash,
            Command::DownloadNow => Input::DownloadNow,
            Command::RemoveDownload => Input::RemoveDownload,
            Command::KeepDownloaded => Input::KeepDownloaded,
            Command::SaveCopy => Input::SaveCopy,
            Command::ShowPreview | Command::HidePreview => Input::TogglePreview,
        })
    }
}

/// What a menu line yields when picked: a command, an app of the Open With submenu, or "Other…".
#[derive(Debug, Clone, PartialEq)]
pub enum Pick {
    /// A command of the menu.
    Command(Command),
    /// An app to open the selection with.
    App(AppPick),
    /// Choose from every installed app.
    OtherApp,
}

impl Pick {
    /// What picking it asks of the browser; `None` for a parent of a submenu.
    pub fn input(&self) -> Option<Input> {
        match self {
            Pick::Command(command) => command.input(),
            Pick::App(app) => Some(Input::OpenWith(app.clone())),
            Pick::OtherApp => Some(Input::ChooseApp),
        }
    }
}

pub use crate::words::OTHER_APP;

/// One app line of the Open With submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppLine {
    /// What it yields.
    pub pick: AppPick,
    /// Its words: the name, and "(default)" after the default's.
    pub title: String,
    /// The image file of its icon, when it has one.
    pub icon: Option<PathBuf>,
}

/// The Open With lines for `apps`: the default first with "(default)" after its name, the others
/// sorted by name (ignoring case).
pub fn app_lines(apps: Vec<AppChoiceView>) -> Vec<AppLine> {
    let (mut defaults, mut others): (Vec<_>, Vec<_>) =
        apps.into_iter().partition(|app| app.is_default);
    others.sort_by_cached_key(|app| (app.name.to_lowercase(), app.name.clone()));
    defaults.truncate(1);
    defaults
        .into_iter()
        .map(|app| AppLine {
            title: words::default_app_title(&app.name),
            pick: app.pick,
            icon: app.icon,
        })
        .chain(others.into_iter().map(|app| AppLine {
            title: app.name,
            pick: app.pick,
            icon: app.icon,
        }))
        .collect()
}

/// Every app for the sheet behind "Other…", sorted by name (ignoring case), with no default mark.
pub fn app_names(apps: Vec<AppChoiceView>) -> Vec<AppLine> {
    let mut lines: Vec<AppLine> = apps
        .into_iter()
        .map(|app| AppLine {
            pick: app.pick,
            title: app.name,
            icon: app.icon,
        })
        .collect();
    lines.sort_by_cached_key(|line| (line.title.to_lowercase(), line.title.clone()));
    lines
}

/// Where the press was made.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Over {
    /// On a row (which the press has selected).
    Row,
    /// On the folder's background.
    Background,
}

/// The menu for a press `over` something in `browser`: groups of commands, each with whether it
/// can be picked now.
pub fn groups_for(browser: &Browser, over: Over) -> Vec<Vec<(Command, bool)>> {
    let selected = browser.selection().len();
    let some = selected > 0;
    let one = selected == 1;
    let rows = !browser.rows().is_empty();
    match (browser.location(), over) {
        (Location::Trash, Over::Row) => vec![vec![
            (Command::PutBack, some),
            (Command::DeleteImmediately, some),
        ]],
        (Location::Trash, Over::Background) => vec![vec![(Command::EmptyTrash, rows)]],
        (_, Over::Row) => {
            let mut groups = vec![open_group(browser, some)];
            groups.extend(cloud_group(browser));
            groups.extend([
                vec![(Command::MoveToTrash, some)],
                vec![
                    (Command::GetInfo, some),
                    (Command::Rename, one),
                    (Command::QuickLook, some),
                ],
                vec![(Command::Copy, some), (Command::CopyAsPathname, some)],
                vec![(Command::NewFolder, true)],
            ]);
            groups
        }
        (_, Over::Background) => vec![
            vec![(Command::NewFolder, true)],
            vec![(Command::GetInfoFolder, true)],
            vec![(Command::Paste, browser.can_paste())],
            vec![(preview_command(browser), true)],
        ],
    }
}

/// Download Now, Remove Download and Keep Downloaded, for a selection in a cloud drive.
fn cloud_group(browser: &Browser) -> Option<Vec<(Command, bool)>> {
    let menu = browser.cloud_menu()?;
    let mut group = vec![
        (Command::DownloadNow, menu.download),
        (Command::RemoveDownload, menu.remove),
        (Command::KeepDownloaded, menu.keep),
    ];
    if menu.copy {
        group.push((Command::SaveCopy, menu.copy));
    }
    Some(group)
}

/// Show Preview while the pane is away, Hide Preview while it is up.
fn preview_command(browser: &Browser) -> Command {
    if browser.pane_shown() {
        Command::HidePreview
    } else {
        Command::ShowPreview
    }
}

/// Open, Open in New Tab (only on a lone folder, right after Open) and Open With.
fn open_group(browser: &Browser, some: bool) -> Vec<(Command, bool)> {
    let mut group = vec![(Command::Open, some)];
    if browser.can_open_in_new_tab() {
        group.push((Command::OpenInNewTab, true));
    }
    group.push((Command::OpenWith, some));
    group
}
