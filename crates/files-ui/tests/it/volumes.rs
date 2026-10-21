//! Disks: the edge lists them, follows hot-plug, opens and ejects them and tells what failed; the
//! sidebar shows them with an eject button.

use std::sync::Arc;
use std::time::Duration;

use ds::prelude::Point;
use ds_harness::Input as Click;
use ds_harness::harness::settle_until;
use files_core::{
    EjectFailure, FolderPath, ProgramName, Volume, VolumeId, VolumeKind, VolumeName, VolumeTrouble,
};
use files_model::{Effect, Input};
use files_ui::edge::Heard;
use files_ui::{FakeVolumes, Launch, VolumeCall};
use tempfile::TempDir;

use super::edge::{next, rig_over};
use super::support::{RecordingOpener, harness, launch, scratch_folder};

fn id(text: &str) -> VolumeId {
    VolumeId::new(text)
}

fn stick() -> Volume {
    Volume::new(id("stick"), VolumeName::new("STICK"), VolumeKind::External)
        .mounted_at(FolderPath::new("/run/media/p/STICK").unwrap())
        .ejectable()
}

fn card() -> Volume {
    Volume::new(id("card"), VolumeName::new("CARD"), VolumeKind::External).ejectable()
}

fn names(heard: &Heard) -> Vec<String> {
    match heard {
        Heard::Input(Input::Volumes(list)) => {
            list.iter().map(|v| v.name.as_str().to_owned()).collect()
        }
        other @ (Heard::Input(_) | Heard::Told(_) | Heard::Announced(_) | Heard::Looked(..)) => {
            panic!("not a list of disks: {other:?}")
        }
    }
}

#[test]
fn the_edge_lists_the_disks_at_the_start_and_again_when_one_is_plugged_in() {
    let fake = Arc::new(FakeVolumes::with(vec![stick()]));
    let mut rig = rig_over(fake.clone());
    assert_eq!(names(&next(&mut rig.inbox)), ["STICK"]);

    fake.set(vec![stick(), card()]);
    assert_eq!(names(&next(&mut rig.inbox)), ["STICK", "CARD"]);
}

#[test]
fn opening_a_disk_mounts_it_and_says_where() {
    let fake = Arc::new(FakeVolumes::with(vec![card()]));
    let mut rig = rig_over(fake.clone());
    let _ = next(&mut rig.inbox);
    rig.edge.run(vec![Effect::MountVolume(id("card"))]);
    let here = FolderPath::new("/run/media/fake/card").unwrap();
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Mounted(id("card"), Ok(here)))
    );
    assert_eq!(fake.calls(), vec![VolumeCall::Mounted(id("card"))]);
}

#[test]
fn ejecting_a_busy_disk_comes_back_as_a_failure_and_the_list_stays() {
    let busy = EjectFailure::InUse(vec![ProgramName::new("vlc")]);
    let fake = Arc::new(FakeVolumes::with(vec![stick()]).failing_eject(busy.clone()));
    let mut rig = rig_over(fake);
    let _ = next(&mut rig.inbox);
    rig.edge.run(vec![Effect::EjectVolume(id("stick"))]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Ejected(id("stick"), Err(busy)))
    );
    assert_eq!(names(&next(&mut rig.inbox)), ["STICK"]);
}

#[test]
fn ejecting_a_disk_takes_it_off_the_list() {
    let fake = Arc::new(FakeVolumes::with(vec![stick(), card()]));
    let mut rig = rig_over(fake);
    let _ = next(&mut rig.inbox);
    rig.edge.run(vec![Effect::EjectVolume(id("stick"))]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::Ejected(id("stick"), Ok(())))
    );
    assert_eq!(names(&next(&mut rig.inbox)), ["CARD"]);
}

#[test]
fn what_went_wrong_is_told_in_finders_words() {
    let mut rig = rig_over(Arc::new(FakeVolumes::default()));
    let _ = next(&mut rig.inbox);
    rig.edge.run(vec![Effect::TellVolume {
        name: VolumeName::new("STICK"),
        trouble: VolumeTrouble::NotEjected(EjectFailure::InUse(Vec::new())),
    }]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Told(
            "The disk \u{201c}STICK\u{201d} wasn\u{2019}t ejected because one or more programs may be using it."
                .to_owned()
        )
    );
}

/// A window whose computer has `volumes`, once the sidebar shows `shows`; the scratch folders it
/// stands on come with it.
fn window_over(
    volumes: Arc<FakeVolumes>,
    shows: &'static str,
) -> (ds_harness::Harness, TempDir, TempDir) {
    let dir = scratch_folder();
    let config = tempfile::tempdir().unwrap();
    let opener = Arc::new(RecordingOpener::default());
    let launch: Launch = launch(dir.path(), config.path(), &opener).with_volumes(volumes);
    let mut harness = harness(launch);
    settle_until(&mut harness, |h| h.html().contains(shows));
    (harness, dir, config)
}

#[test]
fn the_sidebar_lists_the_disks_under_locations_and_the_eject_button_ejects() {
    let fake = Arc::new(FakeVolumes::with(vec![stick(), card()]));
    let (mut harness, _dir, _config) = window_over(fake.clone(), "STICK");
    let html = harness.html();
    for text in ["STICK", "CARD", "Eject STICK", "Eject CARD"] {
        assert!(html.contains(text), "{text} is not in the sidebar:\n{html}");
    }
    let button: Point = harness
        .centre("[aria-label=\"Eject STICK\"]")
        .unwrap_or_else(|| panic!("no eject button:\n{}", harness.html()));
    harness.send(Click::click(button));
    harness.advance(Duration::from_millis(50));
    settle_until(&mut harness, |h| !h.html().contains("STICK"));
    assert_eq!(fake.calls(), vec![VolumeCall::Ejected(id("stick"))]);
    assert!(harness.html().contains("CARD"));
}
