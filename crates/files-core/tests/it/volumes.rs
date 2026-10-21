//! A volume knows which folders are inside it.

use files_core::{FolderPath, Volume, VolumeId, VolumeKind, VolumeName};

fn folder(path: &str) -> FolderPath {
    FolderPath::new(path).unwrap()
}

fn stick() -> Volume {
    Volume::new(
        VolumeId::new("sdb1"),
        VolumeName::new("STICK"),
        VolumeKind::External,
    )
}

#[test]
fn a_volume_holds_its_folder_and_what_is_inside() {
    let mounted = stick().mounted_at(folder("/run/media/p/STICK"));
    let cases = [
        ("the mount itself", "/run/media/p/STICK", true),
        ("a folder inside", "/run/media/p/STICK/a/b", true),
        (
            "a sibling with the same start",
            "/run/media/p/STICK2",
            false,
        ),
        ("elsewhere", "/home/p", false),
    ];
    for (label, path, want) in cases {
        assert_eq!(mounted.holds(&folder(path)), want, "{label}");
    }
}

#[test]
fn an_unmounted_volume_holds_nothing() {
    assert!(!stick().holds(&folder("/")));
    assert_eq!(stick().folder(), None);
}
