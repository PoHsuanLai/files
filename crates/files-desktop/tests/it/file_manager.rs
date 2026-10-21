//! `org.freedesktop.FileManager1` over a private bus.

use std::sync::Arc;
use std::time::Duration;

use files_core::{FileName, FolderPath, Location};
use files_desktop::file_manager::{INTERFACE, OBJECT_PATH};
use files_desktop::{Claim, Reveal, Show, ShowRequest, Sink, StartupId, claim};
use tokio::sync::mpsc::{UnboundedReceiver, unbounded_channel};

use crate::bus::PrivateBus;

pub fn sink() -> (Sink, UnboundedReceiver<Show>) {
    let (sender, receiver) = unbounded_channel();
    let sink: Sink = Arc::new(move |show| {
        let _ = sender.send(show);
    });
    (sink, receiver)
}

pub async fn next(receiver: &mut UnboundedReceiver<Show>) -> Show {
    tokio::time::timeout(Duration::from_secs(5), receiver.recv())
        .await
        .expect("a request within five seconds")
        .expect("the sink is open")
}

pub fn folder(path: &str) -> FolderPath {
    FolderPath::new(path).unwrap()
}

pub fn reveal(dir: &str, name: &str) -> Reveal {
    Reveal {
        folder: folder(dir),
        name: FileName::new(name).unwrap(),
    }
}

async fn call(client: &zbus::Connection, method: &str, uris: &[&str], startup: &str) {
    client
        .call_method(
            Some(INTERFACE),
            OBJECT_PATH,
            Some(INTERFACE),
            method,
            &(uris.to_vec(), startup),
        )
        .await
        .unwrap();
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn show_items_reaches_the_sink_with_each_file_and_the_startup_id() {
    let Some(bus) = PrivateBus::start("show_items") else {
        return;
    };
    let (sink, mut heard) = sink();
    let server = bus.connect().await;
    assert!(matches!(
        claim(&server, sink).await.unwrap(),
        Claim::First(_)
    ));

    let client = bus.connect().await;
    let uris = [
        "file:///tmp/My%20Docs/a%20b.txt",
        "file:///home/me/caf%C3%A9.pdf",
    ];
    call(&client, "ShowItems", &uris, "startup-7").await;

    let show = next(&mut heard).await;
    assert_eq!(
        show.request,
        ShowRequest::Items(vec![
            reveal("/tmp/My Docs", "a b.txt"),
            reveal("/home/me", "café.pdf"),
        ])
    );
    assert_eq!(show.startup, StartupId::new("startup-7"));
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn show_folders_skips_what_it_cannot_show_and_an_empty_startup_id_is_none() {
    let Some(bus) = PrivateBus::start("show_folders") else {
        return;
    };
    let (sink, mut heard) = sink();
    let server = bus.connect().await;
    let _claim = claim(&server, sink).await.unwrap();

    let client = bus.connect().await;
    let uris = [
        "trash:///",
        "file:///tmp/x",
        "https://example.org/",
        "garbage",
        "file://otherhost/srv",
        "file:///var/",
    ];
    call(&client, "ShowFolders", &uris, "").await;

    let show = next(&mut heard).await;
    assert_eq!(
        show.request,
        ShowRequest::Folders(vec![
            Location::Trash,
            Location::Folder(folder("/tmp/x")),
            Location::Folder(folder("/var")),
        ])
    );
    assert_eq!(show.startup, None);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn properties_are_asked_for_and_a_call_with_nothing_to_show_is_silent() {
    let Some(bus) = PrivateBus::start("properties") else {
        return;
    };
    let (sink, mut heard) = sink();
    let server = bus.connect().await;
    let _claim = claim(&server, sink).await.unwrap();

    let client = bus.connect().await;
    call(
        &client,
        "ShowItemProperties",
        &["garbage", "trash:///"],
        "t",
    )
    .await;
    call(&client, "ShowItemProperties", &["file:///tmp/x.txt"], "t").await;

    // Calls are answered in order, so the refused one would have been heard first.
    let show = next(&mut heard).await;
    assert_eq!(
        show.request,
        ShowRequest::Properties(vec![reveal("/tmp", "x.txt")])
    );
    assert!(heard.try_recv().is_err());
}
