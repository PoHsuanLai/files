//! Another program's asks reach the window as model inputs: a fake source sends, the window
//! follows.

use std::cell::RefCell;
use std::time::Duration;

use ds_harness::Harness;
use ds_harness::harness::settle_until;
use files_core::{FileName, FolderPath};
use files_model::Input;
use files_ui::{Query, Reading, Request, RequestSender, channel};
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

/// The window's reading of `query`, waiting for it while the window runs.
fn read(harness: &mut Harness, sender: &RequestSender, query: Query) -> Reading {
    let answer = RefCell::new(sender.ask(query).unwrap());
    let got = RefCell::new(None);
    settle_until(harness, |_| {
        if got.borrow().is_none() {
            *got.borrow_mut() = answer.borrow_mut().try_now();
        }
        got.borrow().is_some()
    });
    got.into_inner().unwrap()
}

#[test]
fn a_quiet_input_selects_and_the_window_reads_the_selection_and_the_folder_back() {
    let (mut harness, sender, dir, _config) = open();
    assert_eq!(
        read(&mut harness, &sender, Query::Selection),
        Reading::Selection(Vec::new())
    );

    sender.send_quietly(Input::Reveal {
        folder: FolderPath::new(dir.path()).unwrap(),
        names: vec![FileName::new("report.pdf").unwrap()],
    });
    settle_until(&mut harness, |h| selected(h, 2));
    let folder = FolderPath::new(dir.path()).unwrap();
    assert_eq!(
        read(&mut harness, &sender, Query::Selection),
        Reading::Selection(vec![files_core::ItemRef::new(
            folder.clone(),
            FileName::new("report.pdf").unwrap()
        )])
    );
    assert_eq!(
        read(&mut harness, &sender, Query::CurrentFolder),
        Reading::Place(files_core::Location::Folder(folder))
    );
}

#[test]
fn the_window_says_there_is_nothing_to_undo_before_it_has_changed_anything() {
    let (mut harness, sender, _dir, _config) = open();
    assert_eq!(
        read(&mut harness, &sender, Query::NextUndo),
        Reading::Undo(None)
    );
}

#[test]
fn a_question_to_a_closed_window_is_an_error() {
    let (sender, requests) = channel();
    drop(requests);
    assert!(sender.ask(Query::Selection).is_err());
}
