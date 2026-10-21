//! Disks in the sidebar: the list as told, opening one, ejecting one, and the answers.

use files_core::{
    Eject, EjectFailure, FolderPath, Location, MountFailure, Volume, VolumeId, VolumeName,
    VolumeTrouble,
};

use crate::browser::Browser;
use crate::effect::Effect;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// The disks the sidebar lists, in the order the caller gave them.
    pub fn volumes(&self) -> &[Volume] {
        &self.volumes
    }

    fn volume(&self, id: &VolumeId) -> Option<&Volume> {
        self.volumes.iter().find(|volume| volume.id == *id)
    }

    fn tell_volume(self, id: &VolumeId, trouble: VolumeTrouble) -> Stepped {
        let name: Option<VolumeName> = self.volume(id).map(|volume| volume.name.clone());
        let effects = name
            .map(|name| vec![Effect::TellVolume { name, trouble }])
            .unwrap_or_default();
        (self, effects)
    }

    /// The disks changed. A window that was inside a disk which is no longer open goes home.
    pub(crate) fn volumes_changed(self, volumes: Vec<Volume>) -> Stepped {
        let left = self.folder().is_some_and(|here| {
            self.volumes.iter().any(|old| {
                old.holds(here)
                    && !volumes
                        .iter()
                        .any(|now| now.id == old.id && now.holds(here))
            })
        });
        let home = self.home.clone();
        let browser = Browser { volumes, ..self };
        match (left, home) {
            (true, Some(home)) => browser.go(Location::Folder(home)),
            _ => (browser, Vec::new()),
        }
    }

    /// Go to a disk, opening it first when it is not open.
    pub(crate) fn open_volume(self, id: VolumeId) -> Stepped {
        let Some(volume) = self.volume(&id) else {
            return (self, Vec::new());
        };
        match volume.folder().cloned() {
            Some(folder) => self.go(Location::Folder(folder)),
            None => (self, vec![Effect::MountVolume(id)]),
        }
    }

    /// A disk was opened, or could not be.
    pub(crate) fn mounted(self, id: VolumeId, result: Result<FolderPath, MountFailure>) -> Stepped {
        match result {
            Ok(folder) => self.go(Location::Folder(folder)),
            Err(failure) => self.tell_volume(&id, VolumeTrouble::NotOpened(failure)),
        }
    }

    /// Eject a disk, when the sidebar offers to.
    pub(crate) fn eject(self, id: VolumeId) -> Stepped {
        match self.volume(&id).map(|volume| volume.eject) {
            Some(Eject::Allowed) => (self, vec![Effect::EjectVolume(id)]),
            Some(Eject::Never | _) | None => (self, Vec::new()),
        }
    }

    /// A disk was ejected, or could not be. The list follows from the caller's next
    /// `Input::Volumes`.
    pub(crate) fn ejected(self, id: VolumeId, result: Result<(), EjectFailure>) -> Stepped {
        match result {
            Ok(()) => (self, Vec::new()),
            Err(failure) => self.tell_volume(&id, VolumeTrouble::NotEjected(failure)),
        }
    }
}
