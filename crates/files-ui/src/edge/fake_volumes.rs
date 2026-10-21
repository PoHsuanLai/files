//! A computer whose disks a test controls.

use std::sync::{Mutex, PoisonError};

use files_core::{Eject, EjectFailure, FolderPath, Mount, MountFailure, Volume, VolumeId};

use super::volumes::{Changed, Volumes, WatchGuard};
use crate::error::VolumeError;

/// What was asked of a [`FakeVolumes`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum VolumeCall {
    /// The disk was opened.
    Mounted(VolumeId),
    /// The disk was ejected.
    Ejected(VolumeId),
    /// The disk was ejected by force.
    ForceEjected(VolumeId),
}

#[derive(Default)]
struct State {
    volumes: Vec<Volume>,
    calls: Vec<VolumeCall>,
    eject_fails: Option<EjectFailure>,
    mount_fails: Option<MountFailure>,
    changed: Option<Changed>,
}

/// Disks kept in memory. Opening one mounts it under `/run/media/fake/`; ejecting one takes it
/// away. [`FakeVolumes::set`] plugs and unplugs disks the way a person would.
#[derive(Default)]
pub struct FakeVolumes {
    state: Mutex<State>,
}

impl FakeVolumes {
    /// A computer with these disks.
    pub fn with(volumes: Vec<Volume>) -> FakeVolumes {
        let fake = FakeVolumes::default();
        fake.lock().volumes = volumes;
        fake
    }

    /// The same computer where every eject fails with `failure`.
    pub fn failing_eject(self, failure: EjectFailure) -> FakeVolumes {
        self.lock().eject_fails = Some(failure);
        self
    }

    /// The same computer where every open fails with `failure`.
    pub fn failing_mount(self, failure: MountFailure) -> FakeVolumes {
        self.lock().mount_fails = Some(failure);
        self
    }

    /// The disks are now these, and anyone watching is told.
    pub fn set(&self, volumes: Vec<Volume>) {
        let mut state = self.lock();
        state.volumes = volumes;
        if let Some(changed) = &state.changed {
            changed();
        }
    }

    /// Everything asked so far, in order.
    pub fn calls(&self) -> Vec<VolumeCall> {
        self.lock().calls.clone()
    }

    fn lock(&self) -> std::sync::MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl std::fmt::Debug for FakeVolumes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FakeVolumes")
    }
}

impl Volumes for FakeVolumes {
    fn list(&self) -> Result<Vec<Volume>, VolumeError> {
        Ok(self.lock().volumes.clone())
    }

    fn mount(&self, id: &VolumeId) -> Result<FolderPath, MountFailure> {
        let mut state = self.lock();
        state.calls.push(VolumeCall::Mounted(id.clone()));
        if let Some(failure) = state.mount_fails {
            return Err(failure);
        }
        let folder = FolderPath::new(format!("/run/media/fake/{}", id.as_str()))
            .map_err(|_| MountFailure::Other)?;
        let volume = state
            .volumes
            .iter_mut()
            .find(|volume| volume.id == *id)
            .ok_or(MountFailure::Gone)?;
        volume.mount = Mount::At(folder.clone());
        Ok(folder)
    }

    fn eject(&self, id: &VolumeId) -> Result<(), EjectFailure> {
        let mut state = self.lock();
        state.calls.push(VolumeCall::Ejected(id.clone()));
        if let Some(failure) = state.eject_fails.clone() {
            return Err(failure);
        }
        take_away(&mut state, id)
    }

    /// Force gets past a busy disk, which is what a failing eject stands for here.
    fn force_eject(&self, id: &VolumeId) -> Result<(), EjectFailure> {
        let mut state = self.lock();
        state.calls.push(VolumeCall::ForceEjected(id.clone()));
        take_away(&mut state, id)
    }

    fn watch(&self, changed: Changed) -> Result<WatchGuard, VolumeError> {
        self.lock().changed = Some(changed);
        Ok(WatchGuard::new(()))
    }
}

/// Takes the disk out of the list if it can be ejected, else says it is gone.
fn take_away(state: &mut State, id: &VolumeId) -> Result<(), EjectFailure> {
    match state.volumes.iter().find(|volume| volume.id == *id) {
        Some(volume) if volume.eject == Eject::Allowed => {}
        Some(_) | None => return Err(EjectFailure::Gone),
    }
    state.volumes.retain(|volume| volume.id != *id);
    Ok(())
}
