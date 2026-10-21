//! What the disk service says about a block device, and which of them are disks for the sidebar.
//! Pure: the facts come in as plain values, a table test goes through them.

use std::path::PathBuf;

use files_core::{FolderPath, Volume, VolumeId, VolumeKind, VolumeName};

/// Where people's own mounts go; a disk mounted anywhere else is the system's.
const USER_MOUNTS: [&str; 3] = ["/run/media", "/media", "/mnt"];

/// The name of a disk with no label.
const UNTITLED: &str = "Untitled";

/// The drive a block device is part of.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct DriveFacts {
    /// The drive can be taken away (a USB stick, a card).
    pub(super) removable: bool,
    /// The drive can eject its media (an optical drive).
    pub(super) ejectable: bool,
    /// The drive can be switched off.
    pub(super) can_power_off: bool,
    /// What media the drive takes (`optical_dvd`, `flash_sd`, ...).
    pub(super) media: Vec<String>,
}

/// What is known of one block device that holds a filesystem.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(super) struct BlockFacts {
    /// Its object path, which names it.
    pub(super) path: String,
    /// The system says nobody should see it.
    pub(super) hint_ignore: bool,
    /// The system says it belongs to the computer's own setup.
    pub(super) hint_system: bool,
    /// The system's name for it.
    pub(super) hint_name: String,
    /// The filesystem's label.
    pub(super) label: String,
    /// Where it is mounted.
    pub(super) mounts: Vec<PathBuf>,
    /// Its drive, when it has one.
    pub(super) drive: Option<DriveFacts>,
}

impl BlockFacts {
    fn optical(&self) -> bool {
        self.drive
            .as_ref()
            .is_some_and(|drive| drive.media.iter().any(|m| m.starts_with("optical")))
    }

    fn removable(&self) -> bool {
        self.drive
            .as_ref()
            .is_some_and(|drive| drive.removable || drive.ejectable)
    }

    fn mounted_by_person(&self) -> bool {
        self.mounts
            .iter()
            .any(|mount| USER_MOUNTS.iter().any(|base| mount.starts_with(base)))
    }

    /// The first mount point that is a folder path.
    pub(super) fn folder(&self) -> Option<FolderPath> {
        self.mounts
            .iter()
            .find_map(|mount| FolderPath::new(mount.clone()).ok())
    }

    /// What the sidebar calls the disk.
    fn name(&self) -> VolumeName {
        [&self.label, &self.hint_name]
            .into_iter()
            .find(|text| !text.is_empty())
            .map_or_else(
                || VolumeName::new(UNTITLED),
                |text| VolumeName::new(text.as_str()),
            )
    }
}

/// The sidebar's disk for a block device, or `None` when it is not one to show: ignored by the
/// system, or the computer's own and not mounted where a person mounts things.
pub(super) fn volume_of(block: &BlockFacts) -> Option<Volume> {
    if block.hint_ignore {
        return None;
    }
    let kind = if block.optical() {
        VolumeKind::Optical
    } else if block.removable() || !block.hint_system {
        VolumeKind::External
    } else if block.mounted_by_person() {
        VolumeKind::Internal
    } else {
        return None;
    };
    let volume = Volume::new(VolumeId::new(block.path.as_str()), block.name(), kind);
    let volume = match block.folder() {
        Some(folder) => volume.mounted_at(folder),
        None => volume,
    };
    Some(match kind {
        VolumeKind::External | VolumeKind::Optical => volume.ejectable(),
        VolumeKind::Internal | _ => volume,
    })
}
