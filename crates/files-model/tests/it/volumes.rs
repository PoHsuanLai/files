//! Disks in the sidebar: the list, opening and ejecting, and what is told when it fails.

use files_core::{
    EjectFailure, FolderPath, Location, MountFailure, ProgramName, Volume, VolumeId, VolumeKind,
    VolumeName, VolumeTrouble,
};
use files_model::{Effect, Input};

use crate::common::*;

fn id(text: &str) -> VolumeId {
    VolumeId::new(text)
}

fn mount_folder(text: &str) -> FolderPath {
    FolderPath::new(text).unwrap()
}

fn stick() -> Volume {
    Volume::new(id("stick"), VolumeName::new("STICK"), VolumeKind::External)
        .mounted_at(mount_folder("/run/media/p/STICK"))
        .ejectable()
}

fn card() -> Volume {
    Volume::new(id("card"), VolumeName::new("CARD"), VolumeKind::External).ejectable()
}

fn system() -> Volume {
    Volume::new(id("data"), VolumeName::new("Data"), VolumeKind::Internal)
        .mounted_at(mount_folder("/mnt/data"))
}

fn told(name: &str, trouble: VolumeTrouble) -> Effect {
    Effect::TellVolume {
        name: VolumeName::new(name),
        trouble,
    }
}

fn with_volumes() -> files_model::Browser {
    let browser = loaded(Vec::new());
    browser
        .step(Input::Volumes(vec![stick(), card(), system()]), at(0))
        .0
}

#[test]
fn the_list_is_what_the_caller_last_told() {
    let browser = with_volumes();
    let names: Vec<&str> = browser.volumes().iter().map(|v| v.name.as_str()).collect();
    assert_eq!(names, ["STICK", "CARD", "Data"]);
}

#[test]
fn opening_and_ejecting_ask_the_caller_only_when_they_make_sense() {
    let cases: Vec<(&str, Input, Vec<Effect>)> = vec![
        (
            "a mounted disk is gone to",
            Input::OpenVolume(id("stick")),
            vec![
                Effect::Unwatch(home()),
                Effect::List(dir("/run/media/p/STICK")),
                Effect::Watch(dir("/run/media/p/STICK")),
                Effect::LoadPrefs(dir("/run/media/p/STICK")),
            ],
        ),
        (
            "an unmounted disk is mounted first",
            Input::OpenVolume(id("card")),
            vec![Effect::MountVolume(id("card"))],
        ),
        ("an unknown disk", Input::OpenVolume(id("nope")), vec![]),
        (
            "an ejectable disk",
            Input::Eject(id("stick")),
            vec![Effect::EjectVolume(id("stick"))],
        ),
        ("a disk that stays", Input::Eject(id("data")), vec![]),
        (
            "an unknown disk is not ejected",
            Input::Eject(id("x")),
            vec![],
        ),
    ];
    for (label, input, want) in cases {
        let (_, effects) = with_volumes().step(input, at(0));
        assert_eq!(effects, want, "{label}");
    }
}

#[test]
fn answers_go_to_the_disk_or_say_what_failed() {
    let busy = EjectFailure::InUse(vec![ProgramName::new("vlc")]);
    let cases: Vec<(&str, Input, Vec<Effect>)> = vec![
        (
            "mounted: go there",
            Input::Mounted(id("card"), Ok(mount_folder("/run/media/p/CARD"))),
            vec![
                Effect::Unwatch(home()),
                Effect::List(dir("/run/media/p/CARD")),
                Effect::Watch(dir("/run/media/p/CARD")),
                Effect::LoadPrefs(dir("/run/media/p/CARD")),
            ],
        ),
        (
            "not mounted: told",
            Input::Mounted(id("card"), Err(MountFailure::NotAllowed)),
            vec![told(
                "CARD",
                VolumeTrouble::NotOpened(MountFailure::NotAllowed),
            )],
        ),
        (
            "ejected: nothing to do",
            Input::Ejected(id("stick"), Ok(())),
            vec![],
        ),
        (
            "busy: told with the programs",
            Input::Ejected(id("stick"), Err(busy.clone())),
            vec![told("STICK", VolumeTrouble::NotEjected(busy))],
        ),
        (
            "a failure for a disk that left is not told",
            Input::Ejected(id("gone"), Err(EjectFailure::Gone)),
            vec![],
        ),
    ];
    for (label, input, want) in cases {
        let (_, effects) = with_volumes().step(input, at(0));
        assert_eq!(effects, want, "{label}");
    }
}

#[test]
fn a_window_inside_a_disk_that_went_goes_home() {
    let inside = Input::Go(dir("/run/media/p/STICK/photos"));
    let (browser, _) = with_volumes()
        .with_home(mount_folder("/home/p"))
        .step(inside, at(0));
    let ejected = vec![card(), system()];
    let (browser, effects) = browser.step(Input::Volumes(ejected.clone()), at(0));
    assert_eq!(
        browser.location(),
        &Location::Folder(mount_folder("/home/p"))
    );
    assert!(effects.contains(&Effect::List(home())), "{effects:?}");

    // Elsewhere the same change only updates the list.
    let (browser, effects) = with_volumes()
        .with_home(mount_folder("/home/p"))
        .step(Input::Volumes(ejected), at(0));
    assert_eq!(browser.location(), &home());
    assert_eq!(effects, vec![]);
}
