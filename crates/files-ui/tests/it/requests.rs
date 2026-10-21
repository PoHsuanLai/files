//! Another program's asks reach the window as model inputs: a fake source sends, the window
//! follows.

use std::time::Duration;

use ds_harness::Harness;
use ds_harness::harness::settle_until;
use files_core::{FileName, FolderPath};
use files_model::Input;
use files_ui::{Request, RequestSender, channel};
use tempfile::TempDir;

use super::support::{RecordingOpener, harness, launch, scratch_folder};

/// The table row `n`.
fn row(n: usize) -> String {
    format!(".ds-vtable-item[*|data-row=\"{n}\"]")
}

/// A window on the scratch folder, listed, and the end that sends it requests.
fn open() -> (Harness, RequestSender, TempDir, TempDir) {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = std::sync::Arc::new(RecordingOpener::default());
    let (sender, requests) = channel();
    let launch = launch(dir.path(), config.path(), &opener).with_requests(requests);
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains("zebra.png"));
    (harness, sender, dir, config)
}

fn reveal(folder: &std::path::Path, names: &[&str]) -> Request {
    Request {
        input: Input::Reveal {
            folder: FolderPath::new(folder).unwrap(),
            names: names.iter().map(|n| FileName::new(n).unwrap()).collect(),
        },
        raise: None,
    }
}

fn selected(harness: &Harness, n: usize) -> bool {
    harness.attr(&row(n), "data-selected").as_deref() == Some("yes")
}

#[test]
fn a_reveal_in_the_folder_shown_selects_the_row() {
    let (mut harness, sender, dir, _config) = open();
    // By name the rows are notes.txt, Projects, report.pdf, zebra.png.
    sender.send(reveal(dir.path(), &["report.pdf"]));
    settle_until(&mut harness, |h| selected(h, 2));
    assert!(!selected(&harness, 0), "{}", harness.html());
    assert!(!selected(&harness, 3), "{}", harness.html());
}

#[test]
fn a_reveal_in_another_folder_goes_there_and_selects_the_row() {
    let (mut harness, sender, dir, _config) = open();
    sender.send(reveal(&dir.path().join("Projects"), &["inner.txt"]));
    settle_until(&mut harness, |h| {
        h.html().contains("inner.txt") && !h.html().contains("zebra.png") && selected(h, 0)
    });
    harness.advance(Duration::from_millis(50));
    assert!(selected(&harness, 0), "{}", harness.html());
}
