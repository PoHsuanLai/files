//! Disks over a private bus with a stand-in for UDisks2: which are listed, opening, ejecting,
//! what is told when a disk is busy, and hot-plug.

use std::sync::Arc;
use std::time::Duration;

use files_core::{EjectFailure, FolderPath, MountFailure, ProgramName, VolumeId, VolumeKind};
use files_desktop::disks::FakeUsers;
use files_desktop::{DiskUsers, Disks, NoUsers};

use crate::bus::PrivateBus;
use crate::udisks::{Device, DriveSpec, ROOT, Shared, plug, serve};

const STICK: DriveSpec = DriveSpec {
    node: "stick",
    removable: true,
    ejectable: false,
    can_power_off: true,
    media: &["flash"],
};

const DISC: DriveSpec = DriveSpec {
    node: "disc",
    removable: true,
    ejectable: true,
    can_power_off: false,
    media: &["optical_dvd"],
};

const NVME: DriveSpec = DriveSpec {
    node: "nvme",
    removable: false,
    ejectable: false,
    can_power_off: false,
    media: &[],
};

fn device(node: &'static str, label: &'static str) -> Device {
    Device {
        node,
        label,
        hint_name: "",
        hint_system: false,
        hint_ignore: false,
        mount: None,
        drive: None,
    }
}

fn devices() -> Vec<Device> {
    vec![
        Device {
            mount: Some("/run/media/p/STICK"),
            drive: Some(STICK),
            ..device("sdb1", "STICK")
        },
        Device {
            hint_name: "CARD",
            ..device("sdc1", "")
        },
        Device {
            drive: Some(DISC),
            ..device("sr0", "Movie")
        },
        Device {
            hint_system: true,
            mount: Some("/"),
            drive: Some(NVME),
            ..device("nvme0n1p2", "root")
        },
        Device {
            hint_system: true,
            mount: Some("/mnt/data"),
            drive: Some(NVME),
            ..device("nvme0n1p3", "Data")
        },
        Device {
            hint_system: true,
            drive: Some(NVME),
            ..device("nvme0n1p4", "unmounted system")
        },
        Device {
            hint_ignore: true,
            ..device("loop0", "ignored")
        },
        Device {
            mount: Some("/run/media/p/NOLABEL"),
            ..device("sdd1", "")
        },
    ]
}

struct Rig {
    disks: Disks,
    world: Shared,
    server: zbus::Connection,
    _bus: PrivateBus,
}

async fn rig(users: Arc<dyn DiskUsers>) -> Option<Rig> {
    let bus = PrivateBus::start("disks")?;
    let server = bus.connect().await;
    let world = serve(&server, &devices()).await;
    let disks = Disks::new(bus.connect().await, users);
    Some(Rig {
        disks,
        world,
        server,
        _bus: bus,
    })
}

fn id(node: &str) -> VolumeId {
    VolumeId::new(format!("{ROOT}/block_devices/{node}"))
}

async fn within<T>(work: impl std::future::Future<Output = T>) -> T {
    tokio::time::timeout(Duration::from_secs(10), work)
        .await
        .expect("the stand-in service answered in time")
}

#[tokio::test]
async fn only_the_disks_a_person_would_see_are_listed() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    let volumes = within(rig.disks.list()).await.unwrap();
    // (name, kind, mounted at, can eject), in the sidebar's order: external, then the
    // computer's own, then discs.
    let got: Vec<(String, VolumeKind, Option<String>, bool)> = volumes
        .iter()
        .map(|v| {
            (
                v.name.as_str().to_owned(),
                v.kind,
                v.folder().map(|f| f.as_path().display().to_string()),
                v.can_eject,
            )
        })
        .collect();
    let mount = |text: &str| Some(text.to_owned());
    let want = vec![
        ("CARD".to_owned(), VolumeKind::External, None, true),
        (
            "STICK".to_owned(),
            VolumeKind::External,
            mount("/run/media/p/STICK"),
            true,
        ),
        (
            "Untitled".to_owned(),
            VolumeKind::External,
            mount("/run/media/p/NOLABEL"),
            true,
        ),
        (
            "Data".to_owned(),
            VolumeKind::Internal,
            mount("/mnt/data"),
            false,
        ),
        ("Movie".to_owned(), VolumeKind::Optical, None, true),
    ];
    assert_eq!(got, want);
}

#[tokio::test]
async fn opening_an_unmounted_disk_mounts_it_and_gives_the_folder() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    let folder = within(rig.disks.mount(&id("sdc1"))).await.unwrap();
    assert_eq!(folder, FolderPath::new("/run/media/p/sdc1").unwrap());
    let volumes = within(rig.disks.list()).await.unwrap();
    let card = volumes.iter().find(|v| v.name.as_str() == "CARD").unwrap();
    assert_eq!(card.mount, Some(folder));
}

#[tokio::test]
async fn opening_a_disk_the_person_may_not_open_says_so() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    rig.world.lock().unwrap().deny = true;
    let failed = within(rig.disks.mount(&id("sdc1"))).await;
    assert_eq!(failed, Err(MountFailure::NotAllowed));
}

#[tokio::test]
async fn ejecting_unmounts_then_switches_the_drive_off() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    within(rig.disks.eject(&id("sdb1"))).await.unwrap();
    let calls = rig.world.lock().unwrap().calls.clone();
    assert_eq!(calls, ["unmount sdb1", "power_off stick"]);
}

#[tokio::test]
async fn a_disc_is_ejected_by_its_drive_and_a_disk_with_no_drive_only_unmounted() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    within(rig.disks.eject(&id("sr0"))).await.unwrap();
    within(rig.disks.eject(&id("sdd1"))).await.unwrap();
    let calls = rig.world.lock().unwrap().calls.clone();
    assert_eq!(calls, ["unmount sr0", "eject disc", "unmount sdd1"]);
}

#[tokio::test]
async fn a_busy_disk_stays_and_the_programs_using_it_come_with_the_failure() {
    let users = FakeUsers(vec![ProgramName::new("vlc")]);
    let Some(rig) = rig(Arc::new(users)).await else {
        return;
    };
    rig.world.lock().unwrap().busy = true;
    let failed = within(rig.disks.eject(&id("sdb1"))).await;
    assert_eq!(
        failed,
        Err(EjectFailure::InUse(vec![ProgramName::new("vlc")]))
    );
    // The drive is not switched off while its disk is open.
    let calls = rig.world.lock().unwrap().calls.clone();
    assert_eq!(calls, ["unmount sdb1"]);
}

#[tokio::test]
async fn forcing_a_busy_disk_unmounts_it_with_the_force_option_then_switches_the_drive_off() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    rig.world.lock().unwrap().busy = true;
    within(rig.disks.force_eject(&id("sdb1"))).await.unwrap();
    let calls = rig.world.lock().unwrap().calls.clone();
    assert_eq!(calls, ["unmount sdb1 force", "power_off stick"]);
}

#[tokio::test]
async fn a_disk_plugged_in_or_taken_out_is_heard() {
    let Some(rig) = rig(Arc::new(NoUsers)).await else {
        return;
    };
    let mut changes = within(rig.disks.changes()).await.unwrap();
    plug(&rig.server, &rig.world, &device("sde1", "NEW")).await;
    assert_eq!(within(changes.next()).await, Some(()));
    let volumes = within(rig.disks.list()).await.unwrap();
    assert!(volumes.iter().any(|v| v.name.as_str() == "NEW"));
}
