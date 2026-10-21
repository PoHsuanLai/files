//! Opening and ejecting a disk.

use std::collections::HashMap;
use std::path::PathBuf;

use files_core::{EjectFailure, FolderPath, MountFailure, ProgramName, VolumeId};

use super::proxies::{DriveProxy, FilesystemProxy};
use super::{BLOCK, DRIVE, Disks, FILESYSTEM, drive_of, mount_points, text_path};
use crate::error::Said;

impl Disks {
    async fn filesystem(&self, id: &VolumeId) -> zbus::Result<FilesystemProxy<'static>> {
        FilesystemProxy::builder(&self.connection)
            .path(id.as_str().to_owned())?
            .build()
            .await
    }

    /// The first folder the disk is mounted at, read from the service now.
    async fn mounted_at(&self, id: &VolumeId) -> Option<FolderPath> {
        let objects = self.objects().await.ok()?;
        let (_, interfaces) = objects
            .iter()
            .find(|(path, _)| path.as_str() == id.as_str())?;
        let (_, props) = interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == FILESYSTEM)?;
        mount_points(props)
            .into_iter()
            .find_map(|mount| FolderPath::new(mount).ok())
    }

    /// Opens the disk, giving the folder it is at. The service asks the person to authorize when
    /// the system's rules say so.
    pub async fn mount(&self, id: &VolumeId) -> Result<FolderPath, MountFailure> {
        let filesystem = self.filesystem(id).await.map_err(|_| MountFailure::Gone)?;
        match filesystem.mount(HashMap::new()).await {
            Ok(path) => FolderPath::new(path).map_err(|_| MountFailure::Other),
            Err(error) => match Said::from_error(&error) {
                Said::AlreadyMounted => self.mounted_at(id).await.ok_or(MountFailure::Other),
                said @ (Said::Busy | Said::NotAllowed | Said::NotMounted | Said::Other) => {
                    Err(said.mount_failure())
                }
            },
        }
    }

    /// Ejects the disk: unmounts it, then switches its drive off or ejects its media. A disk
    /// something has open is left alone, and the programs that were found come with the failure.
    pub async fn eject(&self, id: &VolumeId) -> Result<(), EjectFailure> {
        let filesystem = self.filesystem(id).await.map_err(|_| EjectFailure::Gone)?;
        let mounts = self.mounts_of(id).await;
        match filesystem.unmount(HashMap::new()).await {
            Ok(()) => {}
            Err(error) => match Said::from_error(&error) {
                Said::NotMounted => {}
                said @ (Said::Busy | Said::NotAllowed | Said::AlreadyMounted | Said::Other) => {
                    return Err(said.eject_failure(self.programs_using(mounts).await));
                }
            },
        }
        self.take_away(id).await
    }

    async fn mounts_of(&self, id: &VolumeId) -> Vec<PathBuf> {
        let Ok(objects) = self.objects().await else {
            return Vec::new();
        };
        objects
            .iter()
            .find(|(path, _)| path.as_str() == id.as_str())
            .and_then(|(_, interfaces)| {
                interfaces
                    .iter()
                    .find(|(interface, _)| interface.as_str() == FILESYSTEM)
            })
            .map(|(_, props)| mount_points(props))
            .unwrap_or_default()
    }

    /// The programs using any of `mounts`, searched off the bus's thread.
    async fn programs_using(&self, mounts: Vec<PathBuf>) -> Vec<ProgramName> {
        let users = std::sync::Arc::clone(&self.users);
        let found = tokio::task::spawn_blocking(move || {
            let mut all: Vec<ProgramName> =
                mounts.iter().flat_map(|mount| users.using(mount)).collect();
            all.sort();
            all.dedup();
            all
        })
        .await;
        found.unwrap_or_default()
    }

    /// After the unmount: power the drive off when it can be, else eject its media when it can be.
    async fn take_away(&self, id: &VolumeId) -> Result<(), EjectFailure> {
        let Some(drive) = self.drive_of(id).await else {
            return Ok(());
        };
        let proxy = DriveProxy::builder(&self.connection)
            .path(drive.path)
            .map_err(|_| EjectFailure::Other)?
            .build()
            .await
            .map_err(|_| EjectFailure::Other)?;
        let done = match (drive.can_power_off, drive.ejectable) {
            (true, _) => proxy.power_off(HashMap::new()).await,
            (false, true) => proxy.eject(HashMap::new()).await,
            (false, false) => return Ok(()),
        };
        done.map_err(|error| Said::from_error(&error).eject_failure(Vec::new()))
    }

    /// The drive behind the block device, with what it can do.
    async fn drive_of(&self, id: &VolumeId) -> Option<DriveAt> {
        let objects = self.objects().await.ok()?;
        let (_, interfaces) = objects
            .iter()
            .find(|(path, _)| path.as_str() == id.as_str())?;
        let (_, block) = interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == BLOCK)?;
        let path = text_path(block, "Drive")?;
        let (_, drive_interfaces) = objects.iter().find(|(at, _)| at.as_str() == path)?;
        let (_, props) = drive_interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == DRIVE)?;
        let facts = drive_of(props);
        Some(DriveAt {
            path,
            can_power_off: facts.can_power_off,
            ejectable: facts.ejectable,
        })
    }
}

/// A drive and what it can do after its disks are unmounted.
struct DriveAt {
    path: String,
    can_power_off: bool,
    ejectable: bool,
}
