//! The context menu's commands: what each place offers, and what can be picked.

use std::time::{Duration, SystemTime};

use files_core::{
    ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Location, TrashId, TrashedItem,
};
use files_model::{AppPick, Availability, Browser, Input, Modifier, Now};
use files_ui::menu::{AppLine, Command, OTHER_APP, Over, Pick, app_lines, app_names, groups_for};
use files_ui::{AppChoiceView, AppRole};

use Availability::{Available, Unavailable};

fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

fn step(browser: Browser, input: Input) -> Browser {
    browser.step(input, Now(Duration::ZERO)).0
}

fn folder_browser() -> Browser {
    let here = Location::Folder(FolderPath::new("/home/p").unwrap());
    let (browser, _) = Browser::new(here.clone());
    let rows = vec![
        Entry::new(name("a.txt"), EntryKind::File(ByteSize(1))),
        Entry::new(name("b.txt"), EntryKind::File(ByteSize(1))),
    ];
    step(browser, Input::Listed(here, Ok(rows)))
}

fn trash_browser() -> Browser {
    let (browser, _) = Browser::new(Location::Trash);
    let kind = EntryKind::File(ByteSize(1));
    let original = ItemRef::new(FolderPath::new("/home/p").unwrap(), name("a.txt"));
    let item = TrashedItem::new(
        TrashId::new("/t/info/a.txt.trashinfo"),
        original,
        SystemTime::UNIX_EPOCH,
        kind.clone(),
    );
    let rows = vec![Entry::new(name("a.txt"), kind).in_trash(item)];
    step(browser, Input::Listed(Location::Trash, Ok(rows)))
}

fn click(browser: Browser, text: &str) -> Browser {
    step(browser, Input::Click(name(text), Modifier::None))
}

type Group = Vec<(Command, Availability)>;

#[test]
fn a_row_in_a_folder_offers_what_the_selection_allows_in_finders_order() {
    let one = click(folder_browser(), "a.txt");
    let want: Vec<Group> = vec![
        vec![(Command::Open, Available), (Command::OpenWith, Available)],
        vec![(Command::MoveToTrash, Available)],
        vec![
            (Command::GetInfo, Available),
            (Command::Rename, Available),
            (Command::QuickLook, Available),
        ],
        vec![
            (Command::Copy, Available),
            (Command::CopyAsPathname, Available),
        ],
        vec![(Command::NewFolder, Available)],
    ];
    assert_eq!(groups_for(&one, Over::Row), want);

    // Two selected: nothing to rename.
    let two = step(one, Input::Click(name("b.txt"), Modifier::Toggle));
    let rename = groups_for(&two, Over::Row)[2][1];
    assert_eq!(rename, (Command::Rename, Unavailable));
}

#[test]
fn the_background_of_a_folder_makes_folders_shows_its_info_and_pastes() {
    let copied = step(click(folder_browser(), "a.txt"), Input::Copy);
    let want: Vec<Group> = vec![
        vec![(Command::NewFolder, Available)],
        vec![(Command::GetInfoFolder, Available)],
        vec![(Command::Paste, Available)],
    ];
    assert_eq!(groups_for(&copied, Over::Background), want);
    assert_eq!(
        groups_for(&folder_browser(), Over::Background)[2],
        vec![(Command::Paste, Unavailable)]
    );
}

#[test]
fn the_trash_offers_put_back_and_erasing() {
    let selected = click(trash_browser(), "a.txt");
    let row: Vec<Group> = vec![vec![
        (Command::PutBack, Available),
        (Command::DeleteImmediately, Available),
    ]];
    assert_eq!(groups_for(&selected, Over::Row), row);
    let background: Vec<Group> = vec![vec![(Command::EmptyTrash, Available)]];
    assert_eq!(groups_for(&selected, Over::Background), background);
}

#[test]
fn each_command_asks_the_browser_for_its_input_and_says_its_words() {
    let table = [
        (Command::Open, Input::Open, "Open"),
        (Command::QuickLook, Input::QuickLook, "Quick Look"),
        (Command::GetInfo, Input::GetInfo, "Get Info"),
        (Command::GetInfoFolder, Input::GetInfoFolder, "Get Info"),
        (
            Command::CopyAsPathname,
            Input::CopyAsPathname,
            "Copy as Pathname",
        ),
        (Command::MoveToTrash, Input::MoveToTrash, "Move to Trash"),
        (Command::Rename, Input::BeginRename, "Rename"),
        (Command::Copy, Input::Copy, "Copy"),
        (Command::Paste, Input::Paste, "Paste"),
        (Command::NewFolder, Input::NewFolder, "New Folder"),
        (Command::PutBack, Input::PutBack, "Put Back"),
        (
            Command::DeleteImmediately,
            Input::DeleteImmediately,
            "Delete Immediately\u{2026}",
        ),
        (
            Command::EmptyTrash,
            Input::EmptyTrash,
            "Empty Trash\u{2026}",
        ),
    ];
    for (command, input, title) in table {
        assert_eq!(command.input(), Some(input));
        assert_eq!(command.title(), title);
    }
    // Open With is a parent: it opens a submenu and asks nothing itself.
    assert_eq!(Command::OpenWith.input(), None);
    assert_eq!(Command::OpenWith.title(), "Open With");
}

fn app(id: &str, name: &str, role: AppRole) -> AppChoiceView {
    AppChoiceView {
        pick: AppPick::new(id),
        name: name.to_owned(),
        role,
    }
}

fn line(id: &str, title: &str) -> AppLine {
    AppLine {
        pick: AppPick::new(id),
        title: title.to_owned(),
    }
}

#[test]
fn open_with_lists_the_default_first_and_marks_it_then_the_rest_by_name() {
    let apps = vec![
        app("zed", "Zed", AppRole::Other),
        app("viewer", "Viewer", AppRole::Default),
        app("editor", "editor", AppRole::Other),
        app("calc", "Calculator", AppRole::Other),
    ];
    assert_eq!(
        app_lines(apps),
        vec![
            line("viewer", "Viewer (default)"),
            line("calc", "Calculator"),
            line("editor", "editor"),
            line("zed", "Zed"),
        ]
    );
    // With no default, the apps are just sorted.
    let plain = vec![
        app("b", "Bee", AppRole::Other),
        app("a", "Ant", AppRole::Other),
    ];
    assert_eq!(app_lines(plain), vec![line("a", "Ant"), line("b", "Bee")]);
    assert!(app_lines(Vec::new()).is_empty());
}

#[test]
fn the_other_sheet_lists_every_app_by_name_with_no_default_mark() {
    let apps = vec![
        app("z", "Zed", AppRole::Other),
        app("v", "Viewer", AppRole::Default),
    ];
    assert_eq!(app_names(apps), vec![line("v", "Viewer"), line("z", "Zed")]);
}

#[test]
fn a_menu_pick_asks_the_browser_for_its_input() {
    let table = [
        (Pick::Command(Command::Copy), Some(Input::Copy)),
        (Pick::Command(Command::OpenWith), None),
        (
            Pick::App(AppPick::new("viewer")),
            Some(Input::OpenWith(AppPick::new("viewer"))),
        ),
        (Pick::OtherApp, Some(Input::ChooseApp)),
    ];
    for (pick, input) in table {
        assert_eq!(pick.input(), input);
    }
    assert_eq!(OTHER_APP, "Other\u{2026}");
}
