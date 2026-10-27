//! The cloud service over a private bus, against porter's own fake of it (`syncd-fakes`): the
//! drives, where items stand, the three asks, the news, and the service going away and back.

use std::time::Duration;

use files_core::{
    CloudKind, CloudNews, CloudState, DriveId, DriveState, FolderPath, ItemRef, Keeping,
};
use files_desktop::{DesktopClouds, Storage};
use files_fs::{CloudError, Clouds};
use syncd_fakes::storage::{FakeStorage, World, serve};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::bus::PrivateBus;

const NAME: &str = "org.quire.Sync1";

struct Rig {
    storage: Storage,
    server: Option<zbus::Connection>,
    root: std::path::PathBuf,
    fake: FakeStorage,
    bus: PrivateBus,
    _scratch: tempfile::TempDir,
}

async fn rig() -> Option<Rig> {
    let bus = PrivateBus::start("storage")?;
    let scratch = tempfile::tempdir().unwrap();
    let world = World::plant(scratch.path()).unwrap();
    let fake = FakeStorage::new(world);
    let server = bus.connect().await;
    serve(&server, NAME, fake.clone()).await.unwrap();
    let client = bus.connect().await;
    let storage = Storage::connect(&client).await.unwrap();
    Some(Rig {
        storage,
        server: Some(server),
        root: scratch.path().join("drives"),
        fake,
        bus,
        _scratch: scratch,
    })
}

fn item(rig: &Rig, under: &str) -> ItemRef {
    ItemRef::from_path(&rig.root.join(under)).unwrap()
}

async fn within<T>(work: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), work)
        .await
        .expect("the stand-in service answered in time")
}

/// Starts following and waits until the watch is joined (the first round trip answers).
fn following(storage: &Storage) -> UnboundedReceiver<CloudNews> {
    let (sender, receiver) = unbounded_channel();
    let storage = storage.clone();
    tokio::spawn(async move {
        storage
            .follow(Box::new(move |news| {
                let _ = sender.send(news);
            }))
            .await;
    });
    receiver
}

#[tokio::test]
async fn the_drives_are_listed_with_their_folders_and_room() {
    let Some(rig) = rig().await else { return };
    let drives = within(rig.storage.drives()).await.unwrap();
    let names: Vec<_> = drives.iter().map(|d| d.provider.to_string()).collect();
    assert_eq!(names, ["OneDrive", "Google Drive"]);
    let one = &drives[0];
    assert_eq!(
        one.folder,
        FolderPath::new(rig.root.join("OneDrive")).unwrap()
    );
    assert_eq!(one.state, DriveState::Ready);
    assert_eq!(one.label.as_str(), "ada@example.test");
    assert!(one.usage.used.is_some() && one.usage.total.is_some());
}

#[tokio::test]
async fn items_stand_where_the_service_says_and_unknown_paths_are_left_out() {
    let Some(rig) = rig().await else { return };
    let beach = item(&rig, "OneDrive/Pictures/Beach.jpg");
    let taxes = item(&rig, "OneDrive/Documents/Taxes 2025.pdf");
    let plan = item(&rig, "Google Drive/Projects/Plan.gdoc");
    let nowhere = item(&rig, "OneDrive/Nowhere.txt");
    let rows = within(
        rig.storage
            .items(&[beach.clone(), taxes.clone(), plan.clone(), nowhere]),
    )
    .await
    .unwrap();
    let find = |wanted: &ItemRef| {
        rows.iter()
            .find(|(at, _)| at == wanted)
            .map(|(_, c)| c.clone())
    };
    assert_eq!(rows.len(), 3);
    assert_eq!(find(&beach).unwrap().state, CloudState::Cloud);
    let taxes = find(&taxes).unwrap();
    assert_eq!(
        (taxes.state, taxes.keeping),
        (CloudState::Downloaded, Keeping::Kept)
    );
    let plan = find(&plan).unwrap();
    assert_eq!(plan.kind, CloudKind::Online);
    assert!(plan.url.is_some(), "an online document carries its address");
}

#[tokio::test]
async fn a_download_moves_and_ends_downloaded_and_the_news_says_so() {
    let Some(rig) = rig().await else { return };
    let beach = item(&rig, "OneDrive/Pictures/Beach.jpg");
    let mut news = following(&rig.storage);
    tokio::time::sleep(Duration::from_millis(200)).await;
    let refused = within(rig.storage.download(std::slice::from_ref(&beach)))
        .await
        .unwrap();
    assert!(refused.is_empty());
    let mut seen = Vec::new();
    within(async {
        while let Some(heard) = news.recv().await {
            if let CloudNews::Items(drive, rows) = heard {
                assert_eq!(drive, DriveId::new("onedrive"));
                for (at, details) in rows {
                    if at == beach {
                        seen.push((details.state, details.share));
                        if details.state == CloudState::Downloaded {
                            return;
                        }
                    }
                }
            }
        }
    })
    .await;
    assert_eq!(seen[0].0, CloudState::Downloading);
    assert!(
        seen.iter()
            .any(|(_, share)| share.is_some_and(|s| s.permille() > 0))
    );
    assert_eq!(seen.last().map(|s| s.0), Some(CloudState::Downloaded));
}

#[tokio::test]
async fn keeping_and_removing_are_asked_and_a_refusal_comes_back_in_words() {
    let Some(rig) = rig().await else { return };
    let report = item(&rig, "OneDrive/Documents/Report.docx");
    let plan = item(&rig, "Google Drive/Projects/Plan.gdoc");
    assert!(
        within(
            rig.storage
                .keep(std::slice::from_ref(&report), Keeping::Kept)
        )
        .await
        .unwrap()
        .is_empty()
    );
    let kept = within(rig.storage.items(std::slice::from_ref(&report)))
        .await
        .unwrap();
    assert_eq!(kept[0].1.keeping, Keeping::Kept);
    let refused = within(rig.storage.download(std::slice::from_ref(&plan)))
        .await
        .unwrap();
    assert_eq!(refused.len(), 1);
    assert_eq!(refused[0].item, plan);
    assert!(!refused[0].words.as_str().is_empty());
    let refused = within(rig.storage.remove_download(&[report]))
        .await
        .unwrap();
    assert!(refused.is_empty() || refused[0].words.as_str().contains("kept"));
}

#[tokio::test(flavor = "multi_thread")]
async fn the_service_going_away_is_news_and_no_drives_and_coming_back_is_news_again() {
    let Some(mut rig) = rig().await else { return };
    let clouds = DesktopClouds::new(rig.storage.clone(), tokio::runtime::Handle::current());
    let (sender, mut news) = unbounded_channel();
    let guard = clouds
        .watch(Box::new(move |heard| {
            let _ = sender.send(heard);
        }))
        .unwrap();
    tokio::time::sleep(Duration::from_millis(200)).await;
    let asked = clouds.clone();
    let listed = tokio::task::spawn_blocking(move || asked.drives())
        .await
        .unwrap();
    assert_eq!(listed.unwrap().len(), 2);

    // Closed, not dropped: the stand-in holds a clone of the connection to send its signals, so a
    // drop alone keeps the name.
    if let Some(server) = rig.server.take() {
        server.close().await.unwrap();
    }
    within(async {
        while let Some(heard) = news.recv().await {
            if heard == CloudNews::Drives {
                return;
            }
        }
    })
    .await;
    let asked = clouds.clone();
    let gone = tokio::task::spawn_blocking(move || asked.drives())
        .await
        .unwrap();
    assert_eq!(gone, Err(CloudError::Unavailable));

    let back = rig.bus.connect().await;
    serve(&back, NAME, rig.fake.clone()).await.unwrap();
    within(async {
        while let Some(heard) = news.recv().await {
            if heard == CloudNews::Drives {
                return;
            }
        }
    })
    .await;
    let asked = clouds.clone();
    let again = tokio::task::spawn_blocking(move || asked.drives())
        .await
        .unwrap();
    assert_eq!(again.unwrap().len(), 2);
    drop((guard, back));
}

#[tokio::test]
async fn save_a_copy_makes_files_next_to_the_documents_and_refuses_what_has_no_format() {
    let Some(rig) = rig().await else { return };
    let plan = item(&rig, "Google Drive/Projects/Plan.gdoc");
    let survey = item(&rig, "Google Drive/Projects/Survey.gform");
    let notes = item(&rig, "OneDrive/Documents/Notes.txt");
    let first = within(
        rig.storage
            .save_copy(&[plan.clone(), survey.clone(), notes.clone()]),
    )
    .await
    .unwrap();
    assert_eq!(first.saved.len(), 1);
    assert_eq!(first.saved[0].document, plan);
    assert_eq!(first.saved[0].copy.name.display(), "Plan.docx");
    let refused: Vec<_> = first.refused.iter().map(|r| r.item.clone()).collect();
    assert!(refused.contains(&survey) && refused.contains(&notes));
    let words = &first
        .refused
        .iter()
        .find(|r| r.item == survey)
        .unwrap()
        .words;
    assert_eq!(words.as_str(), "This document can't be saved as a file.");

    let second = within(rig.storage.save_copy(std::slice::from_ref(&plan)))
        .await
        .unwrap();
    assert_eq!(second.saved[0].copy.name.display(), "Plan 2.docx");
}
