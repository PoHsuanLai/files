//! The fake cloud drives behave as the trait promises.

use std::sync::{Arc, Mutex};

use files_core::{
    CloudDrive, CloudItem, CloudKind, CloudNews, CloudState, DriveId, DriveState, FileName,
    FolderPath, ItemRef, Keeping, ProviderName, Share,
};
use files_fs::{CloudCall, CloudError, Clouds, FakeClouds, NO_FILE_FORMAT_WORDS, OFFLINE_WORDS};

fn drive(state: DriveState) -> CloudDrive {
    CloudDrive::new(
        DriveId::new("od"),
        ProviderName::new("OneDrive"),
        FolderPath::new("/c/OneDrive").unwrap(),
    )
    .in_state(state)
}

fn report() -> ItemRef {
    ItemRef::new(
        FolderPath::new("/c/OneDrive").unwrap(),
        FileName::new("Report.docx").unwrap(),
    )
}

fn in_the_cloud() -> CloudItem {
    CloudItem::new(CloudKind::File, CloudState::Cloud)
}

fn fake(state: DriveState) -> FakeClouds {
    FakeClouds::with(vec![drive(state)]).holding(report(), in_the_cloud())
}

fn state_of(clouds: &FakeClouds) -> CloudState {
    clouds.items(&[report()]).unwrap()[0].1.state
}

#[test]
fn a_download_moves_only_when_told_and_ends_downloaded() {
    let clouds = fake(DriveState::Ready);
    assert!(clouds.download(&[report()]).unwrap().is_empty());
    assert_eq!(state_of(&clouds), CloudState::Downloading);
    clouds.advance(&report(), Share::from_ratio(0.5));
    let details = clouds.items(&[report()]).unwrap()[0].1.clone();
    assert_eq!(details.share, Some(Share::from_ratio(0.5)));
    clouds.finish(&report());
    assert_eq!(state_of(&clouds), CloudState::Downloaded);
    assert_eq!(clouds.calls(), vec![CloudCall::Downloaded(vec![report()])]);
}

#[test]
fn an_offline_drive_refuses_a_download_with_words() {
    let clouds = fake(DriveState::Offline);
    let refused = clouds.download(&[report()]).unwrap();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].words.as_str(), OFFLINE_WORDS);
    assert_eq!(state_of(&clouds), CloudState::Cloud);
}

#[test]
fn keeping_downloads_and_removing_a_download_unkeeps() {
    let clouds = fake(DriveState::Ready);
    clouds.keep(&[report()], Keeping::Kept).unwrap();
    let details = clouds.items(&[report()]).unwrap()[0].1.clone();
    assert_eq!(
        (details.state, details.keeping),
        (CloudState::Downloading, Keeping::Kept)
    );
    clouds.remove_download(&[report()]).unwrap();
    let details = clouds.items(&[report()]).unwrap()[0].1.clone();
    assert_eq!(
        (details.state, details.keeping),
        (CloudState::Cloud, Keeping::NotKept)
    );
}

#[test]
fn watchers_hear_changes_and_the_service_going_and_coming() {
    let clouds = fake(DriveState::Ready);
    let heard = Arc::new(Mutex::new(Vec::new()));
    let sink = Arc::clone(&heard);
    let _guard = clouds
        .watch(Box::new(move |news| sink.lock().unwrap().push(news)))
        .unwrap();
    clouds.download(&[report()]).unwrap();
    clouds.set_away(true);
    assert_eq!(clouds.drives(), Err(CloudError::Unavailable));
    clouds.set_away(false);
    assert_eq!(clouds.drives().unwrap().len(), 1);
    let heard = heard.lock().unwrap();
    assert!(matches!(heard[0], CloudNews::Items(..)));
    assert_eq!(heard[1..], [CloudNews::Drives, CloudNews::Drives]);
}

fn at(name: &str) -> ItemRef {
    ItemRef::new(
        FolderPath::new("/c/OneDrive").unwrap(),
        FileName::new(name).unwrap(),
    )
}

#[test]
fn save_a_copy_names_the_files_and_refuses_what_has_no_format() {
    let online = |kind| CloudItem::new(kind, CloudState::Cloud);
    let clouds = FakeClouds::with(vec![drive(DriveState::Ready)])
        .holding(at("Plan.gdoc"), online(CloudKind::Online))
        .holding(at("Survey.gform"), online(CloudKind::Online))
        .holding(at("Report.docx"), online(CloudKind::File));
    let first = clouds.save_copy(&[at("Plan.gdoc")]).unwrap();
    assert_eq!(first.saved[0].copy, at("Plan.docx"));
    let second = clouds.save_copy(&[at("Plan.gdoc")]).unwrap();
    assert_eq!(second.saved[0].copy, at("Plan 2.docx"));
    let refused = clouds
        .save_copy(&[at("Survey.gform"), at("Report.docx")])
        .unwrap();
    assert!(refused.saved.is_empty());
    assert_eq!(refused.refused[0].words.as_str(), NO_FILE_FORMAT_WORDS);
    assert_eq!(refused.refused.len(), 2);
}
