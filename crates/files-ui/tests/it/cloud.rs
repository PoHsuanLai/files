//! Cloud drives in the window: the sidebar's section, a row's badge through a download, Keep
//! Downloaded's check, a drive that needs signing in, the status line, and the service going
//! away and coming back. The service is `FakeClouds`; nothing touches a bus.

use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use ds::base::press::PointerButton;
use ds::prelude::{Point, ShortcutKey};
use ds_harness::harness::settle_until;
use ds_harness::{Harness, Input as Click};
use files_core::{
    AccountLabel, ByteSize, CloudDrive, CloudItem, CloudKind, CloudState, CloudWords, DriveId,
    DriveState, FileName, FolderPath, ItemRef, OnlineUrl, ProviderName, Share, Usage,
};
use files_fs::{CloudCall, FakeClouds};
use files_model::{Effect, Input, TabId};
use files_ui::edge::{Edge, Heard, Seams};
use tempfile::TempDir;

use super::edge::next;
use super::support::{RecordingOpener, harness, launch, scratch_folder};

fn drive(id: &str, provider: &str, label: &str, at: &Path) -> CloudDrive {
    CloudDrive::new(
        DriveId::new(id),
        ProviderName::new(provider),
        FolderPath::new(at).unwrap(),
    )
    .labelled(AccountLabel::new(label))
}

fn item(dir: &Path, name: &str) -> ItemRef {
    ItemRef::new(FolderPath::new(dir).unwrap(), FileName::new(name).unwrap())
}

fn in_the_cloud() -> CloudItem {
    CloudItem::new(CloudKind::File, CloudState::Cloud)
}

fn downloaded() -> CloudItem {
    CloudItem::new(CloudKind::File, CloudState::Downloaded)
}

struct Rig {
    harness: Harness,
    clouds: Arc<FakeClouds>,
    dir: TempDir,
    _config: TempDir,
}

/// A window on the scratch folder, which is the OneDrive's folder; `report.pdf` is in the cloud
/// and `notes.txt` is downloaded.
fn open(extra: Vec<CloudDrive>, state: DriveState) -> Rig {
    let dir = scratch_folder();
    let one = drive("od", "OneDrive", "ada@example.test", dir.path())
        .in_state(state)
        .using(Usage {
            used: Some(ByteSize(3_400_000_000)),
            total: Some(ByteSize(5_000_000_000)),
            local: ByteSize(4_200_000_000),
        });
    let mut drives = vec![one];
    drives.extend(extra);
    let clouds = Arc::new(
        FakeClouds::with(drives)
            .holding(item(dir.path(), "report.pdf"), in_the_cloud())
            .holding(item(dir.path(), "notes.txt"), downloaded()),
    );
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let launch = launch(dir.path(), config.path(), &opener).with_clouds(clouds.clone());
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains("zebra.png"));
    // The cloud thread has joined the news and read the rows once it has badged the one in the
    // cloud, so a change made by the test from here on is heard.
    settle_until(&mut harness, |h| {
        h.count("[aria-label=\"In the cloud\"]") == 1
    });
    Rig {
        harness,
        clouds,
        dir,
        _config: config,
    }
}

impl Rig {
    fn key(&mut self, key: ShortcutKey) {
        self.harness.send(Click::key(key));
        self.harness.advance(Duration::from_millis(50));
    }

    /// The context menu on `report.pdf`, the third row.
    fn menu_on_report(&mut self) {
        let at: Point = self
            .harness
            .centre(".ds-vtable-item[*|data-row=\"2\"]")
            .unwrap();
        self.harness
            .send(Click::press(at, PointerButton::Secondary));
        settle_until(&mut self.harness, |h| h.html().contains("Move to Trash"));
    }

    fn badges(&self, name: &str) -> usize {
        self.harness.count(&format!("[aria-label=\"{name}\"]"))
    }
}

#[test]
fn the_sidebar_lists_the_drives_under_cloud_and_adds_the_account_when_a_provider_repeats() {
    let other = tempfile::tempdir().unwrap();
    let google = drive("gd", "Google Drive", "ada@example.test", other.path());
    let rig = open(vec![google], DriveState::Ready);
    let html = rig.harness.html();
    for text in ["Cloud", "OneDrive", "Google Drive"] {
        assert!(html.contains(text), "{text} is not in the sidebar:\n{html}");
    }
    assert!(!html.contains("OneDrive (ada"), "one account per provider");

    let work = drive("od2", "OneDrive", "work@example.test", other.path());
    let rig = open(vec![work], DriveState::Ready);
    let html = rig.harness.html();
    for text in [
        "OneDrive (ada@example.test)",
        "OneDrive (work@example.test)",
    ] {
        assert!(html.contains(text), "{text} is not in the sidebar:\n{html}");
    }
}

#[test]
fn a_row_in_the_cloud_wears_a_badge_and_a_downloaded_one_none() {
    let mut rig = open(Vec::new(), DriveState::Ready);
    settle_until(&mut rig.harness, |h| {
        h.count("[aria-label=\"In the cloud\"]") == 1
    });
    assert_eq!(rig.badges("In the cloud"), 1, "only report.pdf");
    assert_eq!(rig.badges("Downloading"), 0);
}

#[test]
fn download_now_fills_the_ring_and_leaves_nothing_when_it_is_downloaded() {
    let mut rig = open(Vec::new(), DriveState::Ready);
    settle_until(&mut rig.harness, |h| {
        h.count("[aria-label=\"In the cloud\"]") == 1
    });
    rig.menu_on_report();
    let html = rig.harness.html();
    for text in ["Download Now", "Keep Downloaded"] {
        assert!(html.contains(text), "{text} is not in the menu:\n{html}");
    }
    // Nothing of it is on this computer yet, so there is no download to remove (as in Finder).
    assert!(
        !html.contains("Remove Download"),
        "Remove Download on a file in the cloud"
    );
    // Open, Open With, then Download Now.
    for _ in 0..3 {
        rig.key(ShortcutKey::Down);
    }
    rig.key(ShortcutKey::Enter);
    let report = item(rig.dir.path(), "report.pdf");
    settle_until(&mut rig.harness, |_| !rig.clouds.calls().is_empty());
    assert_eq!(
        rig.clouds.calls(),
        vec![CloudCall::Downloaded(vec![report.clone()])]
    );

    settle_until(&mut rig.harness, |h| {
        h.count("[aria-label=\"Downloading\"]") == 1
    });
    rig.clouds.advance(&report, Share::from_ratio(0.5));
    settle_until(&mut rig.harness, |h| {
        h.html().contains("aria-valuenow=\"50\"")
    });
    rig.clouds.finish(&report);
    settle_until(&mut rig.harness, |h| {
        h.count("[role=\"progressbar\"]") == 0 && h.count("[aria-label=\"Downloading\"]") == 0
    });
    assert_eq!(rig.badges("In the cloud"), 0, "downloaded shows nothing");
}

#[test]
fn keep_downloaded_is_checked_once_the_row_is_kept() {
    let mut rig = open(Vec::new(), DriveState::Ready);
    let report = item(rig.dir.path(), "report.pdf");
    rig.clouds.set_item(
        report,
        CloudItem::new(CloudKind::File, CloudState::Downloaded).kept(),
    );
    settle_until(&mut rig.harness, |h| h.html().contains("zebra.png"));
    rig.harness.advance(Duration::from_millis(200));
    rig.menu_on_report();
    settle_until(&mut rig.harness, |h| {
        h.count("[role=\"menuitemcheckbox\"][aria-checked=\"true\"]") == 1
    });
}

#[test]
fn a_drive_that_needs_signing_in_says_why_and_offers_accounts_only_when_pressed() {
    let empty = tempfile::tempdir().unwrap();
    let words = "Sign in again to see this drive\u{2019}s files.";
    let signed_out = drive("od", "OneDrive", "ada@example.test", empty.path())
        .in_state(DriveState::NeedsSignIn)
        .saying(CloudWords::new(words));
    let clouds = Arc::new(FakeClouds::with(vec![signed_out]));
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let launch = launch(empty.path(), config.path(), &opener).with_clouds(clouds);
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains("Sign in again"));
    let html = harness.html();
    assert!(html.contains("Open Accounts"), "{html}");
    assert!(!html.contains("Accounts can"), "nothing opens by itself");

    let button: Point = harness.centre(".ds-empty-state-action .ds-button").unwrap();
    harness.send(Click::click(button));
    settle_until(&mut harness, |h| h.html().contains("Accounts can"));
}

#[test]
fn the_status_line_says_what_is_on_this_computer_and_the_room_used() {
    let rig = open(Vec::new(), DriveState::Ready);
    let html = rig.harness.html();
    assert!(html.contains("4.2 GB on this computer"), "{html}");
    assert!(html.contains("3.4 GB of 5 GB used"), "{html}");
}

#[test]
fn a_mark_follows_the_drives_state_in_the_sidebar() {
    let mut rig = open(Vec::new(), DriveState::Ready);
    assert_eq!(rig.harness.count("[data-trailing=\"glyph\"]"), 0);
    let dir = rig.dir.path().to_path_buf();
    let offline = drive("od", "OneDrive", "ada@example.test", &dir).in_state(DriveState::Offline);
    rig.clouds.set_drives(vec![offline]);
    settle_until(&mut rig.harness, |h| {
        h.count("[data-trailing=\"glyph\"]") == 1
    });
}

#[test]
fn a_service_that_goes_away_shows_no_drives_and_the_drives_return_with_it() {
    let mut rig = open(Vec::new(), DriveState::Ready);
    assert!(rig.harness.html().contains("OneDrive"));
    rig.clouds.set_away(true);
    settle_until(&mut rig.harness, |h| !h.html().contains("OneDrive"));
    assert!(rig.harness.html().contains("zebra.png"), "the folder stays");
    rig.clouds.set_away(false);
    settle_until(&mut rig.harness, |h| h.html().contains("OneDrive"));
}

fn edge_over(clouds: Arc<FakeClouds>) -> (Edge, tokio::sync::mpsc::UnboundedReceiver<Heard>) {
    let seams = Seams::disk(Arc::new(RecordingOpener::default())).with_clouds(clouds);
    Edge::new(seams)
}

#[test]
fn the_edge_lists_the_drives_reads_items_and_tells_a_download_then_its_news() {
    let at = Path::new("/c/OneDrive");
    let report = item(at, "report.pdf");
    let fake = Arc::new(
        FakeClouds::with(vec![drive("od", "OneDrive", "", at)])
            .holding(report.clone(), in_the_cloud()),
    );
    let (edge, mut inbox) = edge_over(fake.clone());
    let Heard::All(Input::Clouds(listed)) = next(&mut inbox) else {
        panic!("the drives are told first");
    };
    assert_eq!(listed.len(), 1);

    edge.run_in(TabId::FIRST, vec![Effect::ReadCloud(vec![report.clone()])]);
    assert_eq!(
        next(&mut inbox),
        Heard::Tab(
            TabId::FIRST,
            Input::CloudItems(vec![(report.clone(), in_the_cloud())])
        )
    );

    edge.run_in(
        TabId::FIRST,
        vec![Effect::CloudDownload(vec![report.clone()])],
    );
    assert_eq!(
        next(&mut inbox),
        Heard::Tab(TabId::FIRST, Input::CloudAnswered(Vec::new()))
    );
    let Heard::All(Input::CloudItems(rows)) = next(&mut inbox) else {
        panic!("the download is news to every tab");
    };
    assert_eq!(rows[0].1.state, CloudState::Downloading);

    fake.set_away(true);
    assert_eq!(next(&mut inbox), Heard::All(Input::Clouds(Vec::new())));
}

#[test]
fn an_ask_the_service_cannot_take_is_told_as_a_refusal_in_words() {
    let at = Path::new("/c/OneDrive");
    let report = item(at, "report.pdf");
    let fake = Arc::new(FakeClouds::with(vec![drive("od", "OneDrive", "", at)]));
    let (edge, mut inbox) = edge_over(fake.clone());
    let _ = next(&mut inbox);
    fake.set_away(true);
    assert_eq!(next(&mut inbox), Heard::All(Input::Clouds(Vec::new())));
    edge.run_in(TabId::FIRST, vec![Effect::CloudDownload(vec![report])]);
    let Heard::Tab(_, Input::CloudAnswered(refused)) = next(&mut inbox) else {
        panic!("an answer");
    };
    assert_eq!(refused.len(), 1);
    assert!(refused[0].words.as_str().contains("can't be reached"));
}

fn row(n: usize) -> String {
    format!(".ds-vtable-item[*|data-row=\"{n}\"]")
}

#[test]
fn save_a_copy_is_in_the_menu_of_an_online_row_only_and_selects_the_new_file() {
    let dir = scratch_folder();
    std::fs::write(dir.path().join("Plan.gdoc"), "{}").unwrap();
    let plan = item(dir.path(), "Plan.gdoc");
    let online = CloudItem::new(CloudKind::Online, CloudState::Cloud)
        .opening_at(OnlineUrl::new("https://docs.example.test/d/1").unwrap());
    let clouds = Arc::new(
        FakeClouds::with(vec![drive("od", "OneDrive", "", dir.path())])
            .holding(plan.clone(), online)
            .holding(item(dir.path(), "report.pdf"), in_the_cloud()),
    );
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let mut harness =
        harness(launch(dir.path(), config.path(), &opener).with_clouds(clouds.clone()));
    settle_until(&mut harness, |h| {
        h.html().contains("zebra.png") && h.count("[aria-label=\"In the cloud\"]") == 1
    });
    // notes.txt, Plan.gdoc, Projects, report.pdf, zebra.png.
    let menu_on = |harness: &mut Harness, n: usize| {
        let at: Point = harness.centre(&row(n)).unwrap();
        harness.send(Click::press(at, PointerButton::Secondary));
        settle_until(harness, |h| h.html().contains("Move to Trash"));
    };
    menu_on(&mut harness, 3);
    assert!(
        !harness.html().contains("Save a Copy"),
        "report.pdf is a plain file"
    );
    harness.send(Click::key(ShortcutKey::Escape));
    settle_until(&mut harness, |h| h.count(".ds-menu") == 0);

    menu_on(&mut harness, 1);
    assert!(harness.html().contains("Save a Copy"));
    // The copy the service makes: a file next to the document.
    std::fs::write(dir.path().join("Plan.docx"), "docx").unwrap();
    // Open, Open With, Save a Copy (the rest are dimmed away for an online document).
    for _ in 0..3 {
        harness.send(Click::key(ShortcutKey::Down));
        harness.advance(Duration::from_millis(50));
    }
    harness.send(Click::key(ShortcutKey::Enter));
    settle_until(&mut harness, |_| !clouds.calls().is_empty());
    assert_eq!(clouds.calls(), vec![CloudCall::Copied(vec![plan])]);
    // notes.txt, Plan.docx, Plan.gdoc, ...: the new file is the selected row.
    settle_until(&mut harness, |h| {
        h.attr(&row(1), "data-selected").as_deref() == Some("yes")
    });
    assert_ne!(
        harness.attr(&row(2), "data-selected").as_deref(),
        Some("yes")
    );
}
