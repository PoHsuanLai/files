//! What the context menu offers: pure, so a table can say it.

use files_core::{Location, TagColour};
use files_model::{AppPick, Availability, Browser, Held, Input};

use crate::edge::{AppChoiceView, AppRole};

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
    /// Tag the selection with a colour, picked from a submenu. It is a parent, never picked itself.
    Tags,
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
}

impl Command {
    /// The menu item's words.
    pub fn title(self) -> &'static str {
        match self {
            Command::Open => "Open",
            Command::OpenInNewTab => "Open in New Tab",
            Command::OpenWith => "Open With",
            Command::QuickLook => "Quick Look",
            Command::GetInfo | Command::GetInfoFolder => "Get Info",
            Command::CopyAsPathname => "Copy as Pathname",
            Command::Tags => "Tags",
            Command::MoveToTrash => "Move to Trash",
            Command::Rename => "Rename",
            Command::Copy => "Copy",
            Command::Paste => "Paste",
            Command::NewFolder => "New Folder",
            Command::PutBack => "Put Back",
            Command::DeleteImmediately => "Delete Immediately\u{2026}",
            Command::EmptyTrash => "Empty Trash\u{2026}",
        }
    }

    /// What picking it asks of the browser; `None` for a parent of a submenu, which is not picked.
    pub fn input(self) -> Option<Input> {
        Some(match self {
            Command::OpenWith | Command::Tags => return None,
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
    /// A colour of the Tags submenu: put it on the selection, or take it off.
    Tag(TagColour),
}

impl Pick {
    /// What picking it asks of the browser; `None` for a parent of a submenu.
    pub fn input(&self) -> Option<Input> {
        match self {
            Pick::Command(command) => command.input(),
            Pick::App(app) => Some(Input::OpenWith(app.clone())),
            Pick::OtherApp => Some(Input::ChooseApp),
            Pick::Tag(colour) => Some(Input::Tag(*colour)),
        }
    }
}

/// How the Tags submenu marks `colour`: checked when every selected item has it, a dash when some
/// do.
pub fn tag_mark(browser: &Browser, colour: TagColour) -> Option<TagMark> {
    match browser.held(colour) {
        Held::Every => Some(TagMark::Every),
        Held::Part => Some(TagMark::Part),
        Held::Nothing => None,
    }
}

/// The mark beside a colour in the Tags submenu.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagMark {
    /// Every selected item has the tag.
    Every,
    /// Some of them do.
    Part,
}

/// The words of the line that opens the sheet of every installed app.
pub const OTHER_APP: &str = "Other\u{2026}";

/// One app line of the Open With submenu.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppLine {
    /// What it yields.
    pub pick: AppPick,
    /// Its words: the name, and "(default)" after the default's.
    pub title: String,
}

/// The Open With lines for `apps`: the default first with "(default)" after its name, the others
/// sorted by name (ignoring case).
pub fn app_lines(apps: Vec<AppChoiceView>) -> Vec<AppLine> {
    let (mut defaults, mut others): (Vec<_>, Vec<_>) = apps
        .into_iter()
        .partition(|app| app.role == AppRole::Default);
    others.sort_by_cached_key(|app| (app.name.to_lowercase(), app.name.clone()));
    defaults.truncate(1);
    defaults
        .into_iter()
        .map(|app| AppLine {
            title: format!("{} (default)", app.name),
            pick: app.pick,
        })
        .chain(others.into_iter().map(|app| AppLine {
            title: app.name,
            pick: app.pick,
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
pub fn groups_for(browser: &Browser, over: Over) -> Vec<Vec<(Command, Availability)>> {
    let selected = browser.selection().len();
    let some = if selected > 0 {
        Availability::Available
    } else {
        Availability::Unavailable
    };
    let one = if selected == 1 {
        Availability::Available
    } else {
        Availability::Unavailable
    };
    let rows = if browser.rows().is_empty() {
        Availability::Unavailable
    } else {
        Availability::Available
    };
    match (browser.location(), over) {
        (Location::Trash, Over::Row) => vec![vec![
            (Command::PutBack, some),
            (Command::DeleteImmediately, some),
        ]],
        (Location::Trash, Over::Background) => vec![vec![(Command::EmptyTrash, rows)]],
        (_, Over::Row) => vec![
            open_group(browser, some),
            vec![(Command::MoveToTrash, some)],
            vec![
                (Command::GetInfo, some),
                (Command::Rename, one),
                (Command::QuickLook, some),
            ],
            vec![(Command::Copy, some), (Command::CopyAsPathname, some)],
            vec![(Command::Tags, some)],
            vec![(Command::NewFolder, Availability::Available)],
        ],
        (_, Over::Background) => vec![
            vec![(Command::NewFolder, Availability::Available)],
            vec![(Command::GetInfoFolder, Availability::Available)],
            vec![(Command::Paste, browser.can_paste())],
        ],
    }
}

/// Open, Open in New Tab (only on a lone folder, right after Open) and Open With.
fn open_group(browser: &Browser, some: Availability) -> Vec<(Command, Availability)> {
    let mut group = vec![(Command::Open, some)];
    if browser.can_open_in_new_tab() == Availability::Available {
        group.push((Command::OpenInNewTab, Availability::Available));
    }
    group.push((Command::OpenWith, some));
    group
}
