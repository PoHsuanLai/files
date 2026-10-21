//! The context menu's commands: what each place offers, and what can be picked.

use std::time::{Duration, SystemTime};

use files_core::{
    ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Location, TrashId, TrashedItem,
};
use files_model::{Availability, Browser, Input, Modifier, Now};
use files_ui::menu::{Command, Over, groups_for};

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
fn a_row_in_a_folder_offers_what_the_selection_allows() {
    let one = click(folder_browser(), "a.txt");
    let want: Vec<Group> = vec![
        vec![(Command::Open, Available)],
        vec![
            (Command::MoveToTrash, Available),
            (Command::Rename, Available),
        ],
        vec![(Command::Copy, Available), (Command::Paste, Unavailable)],
        vec![(Command::NewFolder, Available)],
    ];
    assert_eq!(groups_for(&one, Over::Row), want);

    // Two selected: nothing to rename.
    let two = step(one, Input::Click(name("b.txt"), Modifier::Toggle));
    let rename = groups_for(&two, Over::Row)[1][1];
    assert_eq!(rename, (Command::Rename, Unavailable));
}

#[test]
fn the_background_of_a_folder_makes_folders_and_pastes() {
    let copied = step(click(folder_browser(), "a.txt"), Input::Copy);
    let want: Vec<Group> = vec![
        vec![(Command::NewFolder, Available)],
        vec![(Command::Paste, Available)],
    ];
    assert_eq!(groups_for(&copied, Over::Background), want);
    assert_eq!(
        groups_for(&folder_browser(), Over::Background)[1],
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
        assert_eq!(command.input(), input);
        assert_eq!(command.title(), title);
    }
}
