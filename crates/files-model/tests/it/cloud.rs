//! Cloud drives in the browser: asking where rows stand, opening what is not downloaded yet,
//! online documents, the context menu and what a refusal says.

use files_core::{
    CloudDrive, CloudItem, CloudKind, CloudState, CloudWords, DriveId, DriveState, FolderPath,
    ItemRef, Keeping, OnlineUrl, ProviderName, Refused, Share,
};
use files_model::{
    Availability, Browser, CloudMenu, Effect, Input, KeptMark, OFFLINE_WORDS, OpenTarget,
};

use crate::common::*;

fn drive(state: DriveState) -> CloudDrive {
    CloudDrive::new(
        DriveId::new("od"),
        ProviderName::new("OneDrive"),
        FolderPath::new("/c/OneDrive").unwrap(),
    )
    .in_state(state)
}

fn item(text: &str) -> ItemRef {
    ItemRef::new(FolderPath::new("/c/OneDrive").unwrap(), name(text))
}

fn target(text: &str) -> OpenTarget {
    OpenTarget::new(FolderPath::new("/c/OneDrive").unwrap(), name(text))
}

fn file_in(state: CloudState) -> CloudItem {
    CloudItem::new(CloudKind::File, state)
}

/// A browser in the drive's folder holding `Report.docx`, `Notes.txt`, `Plan.gdoc` and `Docs`,
/// with the drive known and the rows read as `states` say.
fn in_drive(state: DriveState, states: Vec<(&str, CloudItem)>) -> Browser {
    let here = dir("/c/OneDrive");
    let (browser, _) = Browser::new(here.clone());
    let entries = vec![
        file("Report.docx"),
        file("Notes.txt"),
        file("Plan.gdoc"),
        folder("Docs"),
    ];
    let (browser, _) = browser.step(Input::Clouds(vec![drive(state)]), at(0));
    let (browser, _) = browser.step(Input::Listed(here, Ok(entries)), at(0));
    let read = states.into_iter().map(|(n, c)| (item(n), c)).collect();
    browser.step(Input::CloudItems(read), at(0)).0
}

fn open(browser: Browser, text: &str) -> (Browser, Vec<Effect>) {
    run(
        browser,
        vec![
            Input::Click(name(text), files_model::Modifier::None),
            Input::Open,
        ],
    )
}

#[test]
fn a_listing_in_a_drive_asks_where_its_rows_stand() {
    let here = dir("/c/OneDrive");
    let (browser, _) = Browser::new(here.clone());
    let (browser, _) = browser.step(Input::Clouds(vec![drive(DriveState::Ready)]), at(0));
    let entries = vec![file("b"), file("a")];
    let (_, effects) = browser.step(Input::Listed(here, Ok(entries)), at(0));
    assert_eq!(
        effects,
        vec![Effect::ReadCloud(vec![item("a"), item("b")])],
        "in the order shown"
    );
}

#[test]
fn a_folder_outside_every_drive_asks_nothing() {
    let (browser, _) = Browser::new(home());
    let (browser, _) = browser.step(Input::Clouds(vec![drive(DriveState::Ready)]), at(0));
    let (_, effects) = browser.step(Input::Listed(home(), Ok(vec![file("a")])), at(0));
    assert!(effects.is_empty());
}

#[test]
fn a_drive_that_arrives_while_its_folder_is_shown_asks_then() {
    let here = dir("/c/OneDrive");
    let (browser, _) = Browser::new(here.clone());
    let (browser, _) = browser.step(Input::Listed(here, Ok(vec![file("a")])), at(0));
    let (_, effects) = browser.step(Input::Clouds(vec![drive(DriveState::Ready)]), at(0));
    assert_eq!(effects, vec![Effect::ReadCloud(vec![item("a")])]);
}

#[test]
fn opening_a_cloud_only_file_downloads_it_and_opens_it_when_it_is_downloaded() {
    let browser = in_drive(
        DriveState::Ready,
        vec![("Report.docx", file_in(CloudState::Cloud))],
    );
    let (browser, effects) = open(browser, "Report.docx");
    assert_eq!(
        effects,
        vec![Effect::CloudDownload(vec![item("Report.docx")])]
    );

    let moving = file_in(CloudState::Downloading).moving(Share::from_ratio(0.4));
    let (browser, effects) = browser.step(
        Input::CloudItems(vec![(item("Report.docx"), moving)]),
        at(0),
    );
    assert!(effects.is_empty(), "nothing opens while it moves");
    assert_eq!(
        browser.cloud_of(&name("Report.docx")).map(|c| c.state),
        Some(CloudState::Downloading)
    );

    let done = file_in(CloudState::Downloaded);
    let (_, effects) = browser.step(Input::CloudItems(vec![(item("Report.docx"), done)]), at(0));
    assert_eq!(
        effects,
        vec![Effect::OpenFiles(vec![target("Report.docx")])]
    );
}

#[test]
fn a_downloaded_file_opens_at_once_and_so_does_one_not_read_yet() {
    let browser = in_drive(
        DriveState::Ready,
        vec![("Notes.txt", file_in(CloudState::Downloaded))],
    );
    let (_, effects) = open(browser.clone(), "Notes.txt");
    assert_eq!(effects, vec![Effect::OpenFiles(vec![target("Notes.txt")])]);
    let (_, effects) = open(browser, "Report.docx");
    assert_eq!(
        effects,
        vec![Effect::OpenFiles(vec![target("Report.docx")])]
    );
}

#[test]
fn opening_while_the_drive_is_offline_says_it_will_download_when_connected() {
    let browser = in_drive(
        DriveState::Offline,
        vec![("Report.docx", file_in(CloudState::Cloud))],
    );
    let (_, effects) = open(browser, "Report.docx");
    assert_eq!(effects, vec![Effect::Say(OFFLINE_WORDS.to_owned())]);
}

#[test]
fn an_online_document_opens_its_address_and_is_never_downloaded() {
    let url = OnlineUrl::new("https://docs.example.test/d/1").unwrap();
    let plan = CloudItem::new(CloudKind::Online, CloudState::Cloud).opening_at(url.clone());
    let browser = in_drive(DriveState::Ready, vec![("Plan.gdoc", plan)]);
    let (browser, effects) = open(browser, "Plan.gdoc");
    assert_eq!(effects, vec![Effect::OpenUrl(url)]);
    let (_, effects) = browser.step(Input::DownloadNow, at(0));
    assert!(effects.is_empty());
}

#[test]
fn a_refusal_is_told_once_per_reason_and_stops_the_wait() {
    let browser = in_drive(
        DriveState::Ready,
        vec![
            ("Report.docx", file_in(CloudState::Cloud)),
            ("Notes.txt", file_in(CloudState::Cloud)),
        ],
    );
    let (browser, _) = open(browser, "Report.docx");
    let why = CloudWords::new("This item isn't in a cloud drive.");
    let refused = vec![
        Refused {
            item: item("Report.docx"),
            words: why.clone(),
        },
        Refused {
            item: item("Notes.txt"),
            words: why,
        },
    ];
    let (browser, effects) = browser.step(Input::CloudAnswered(refused), at(0));
    assert_eq!(
        effects,
        vec![Effect::Say("This item isn't in a cloud drive.".to_owned())]
    );
    let done = file_in(CloudState::Downloaded);
    let (_, effects) = browser.step(Input::CloudItems(vec![(item("Report.docx"), done)]), at(0));
    assert!(effects.is_empty(), "it no longer waits to open");
}

#[test]
fn a_download_that_failed_is_told_in_its_words() {
    let browser = in_drive(
        DriveState::Ready,
        vec![("Report.docx", file_in(CloudState::Cloud))],
    );
    let (browser, _) = open(browser, "Report.docx");
    let failed =
        file_in(CloudState::Error).failed(CloudWords::new("The server could not send it."));
    let (_, effects) = browser.step(
        Input::CloudItems(vec![(item("Report.docx"), failed)]),
        at(0),
    );
    assert_eq!(
        effects,
        vec![Effect::Say("The server could not send it.".to_owned())]
    );
}

#[test]
fn the_menu_offers_what_the_selection_can_do() {
    use Availability::{Available as Yes, Unavailable as No};
    let states = vec![
        ("Report.docx", file_in(CloudState::Cloud)),
        ("Notes.txt", file_in(CloudState::Downloaded).kept()),
        (
            "Plan.gdoc",
            CloudItem::new(CloudKind::Online, CloudState::Cloud),
        ),
    ];
    let cases = [
        (
            "cloud-only",
            "Report.docx",
            Some((Yes, No, Yes, KeptMark::Nothing)),
        ),
        (
            "downloaded and kept",
            "Notes.txt",
            Some((No, Yes, Yes, KeptMark::All)),
        ),
        ("online", "Plan.gdoc", Some((No, No, No, KeptMark::Nothing))),
    ];
    for (label, row, want) in cases {
        let browser = in_drive(DriveState::Ready, states.clone());
        let (browser, _) =
            browser.step(Input::Click(name(row), files_model::Modifier::None), at(0));
        let got = browser
            .cloud_menu()
            .map(|m: CloudMenu| (m.download, m.remove, m.keep, m.kept));
        assert_eq!(got, want, "{label}");
    }
    let browser = in_drive(DriveState::Ready, states);
    assert_eq!(browser.cloud_menu(), None, "nothing selected");
}

#[test]
fn keep_downloaded_turns_on_unless_all_of_the_selection_is_kept() {
    let states = vec![
        ("Report.docx", file_in(CloudState::Cloud)),
        ("Notes.txt", file_in(CloudState::Downloaded).kept()),
    ];
    let cases = [
        (
            "one not kept",
            vec!["Report.docx", "Notes.txt"],
            Keeping::Kept,
        ),
        ("all kept", vec!["Notes.txt"], Keeping::NotKept),
    ];
    for (label, rows, keeping) in cases {
        let browser = in_drive(DriveState::Ready, states.clone());
        let clicks = rows
            .iter()
            .map(|r| Input::Click(name(r), files_model::Modifier::Toggle))
            .collect();
        let (browser, _) = run(browser, clicks);
        let (_, effects) = browser.step(Input::KeepDownloaded, at(0));
        let [Effect::CloudKeep(items, got)] = effects.as_slice() else {
            panic!("{label}: {effects:?}");
        };
        assert_eq!(*got, keeping, "{label}");
        assert_eq!(items.len(), rows.len(), "{label}");
    }
}

#[test]
fn going_to_another_folder_forgets_where_the_rows_stood() {
    let browser = in_drive(
        DriveState::Ready,
        vec![("Report.docx", file_in(CloudState::Cloud))],
    );
    let (browser, _) = browser.step(Input::Go(home()), at(0));
    assert!(browser.cloud_of(&name("Report.docx")).is_none());
    assert_eq!(browser.clouds().len(), 1, "the drives stay");
}
