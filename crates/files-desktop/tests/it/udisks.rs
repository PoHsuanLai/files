//! A stand-in for UDisks2 on a private bus: block devices, drives and the object manager, with the
//! calls recorded. Nothing here touches a real disk or the system bus.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, PoisonError};

use zbus::object_server::SignalEmitter;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::{Connection, interface};

pub const ROOT: &str = "/org/freedesktop/UDisks2";

/// A drive of a device.
#[derive(Clone)]
pub struct DriveSpec {
    pub node: &'static str,
    pub removable: bool,
    pub ejectable: bool,
    pub can_power_off: bool,
    pub media: &'static [&'static str],
}

/// A block device with a filesystem.
#[derive(Clone)]
pub struct Device {
    pub node: &'static str,
    pub label: &'static str,
    pub hint_name: &'static str,
    pub hint_system: bool,
    pub hint_ignore: bool,
    pub mount: Option<&'static str>,
    pub drive: Option<DriveSpec>,
}

/// What the fake has been asked, and how it answers.
#[derive(Default)]
pub struct World {
    pub mounted: HashMap<&'static str, Option<String>>,
    pub calls: Vec<String>,
    pub busy: bool,
    pub deny: bool,
}

pub type Shared = Arc<Mutex<World>>;

fn locked(world: &Shared) -> std::sync::MutexGuard<'_, World> {
    world.lock().unwrap_or_else(PoisonError::into_inner)
}

/// What the service answers with when it refuses.
#[derive(Debug, zbus::DBusError)]
#[zbus(prefix = "org.freedesktop.UDisks2.Error")]
pub enum Refused {
    #[zbus(error)]
    ZBus(zbus::Error),
    DeviceBusy(String),
    NotAuthorized(String),
}

struct Block {
    device: Device,
}

#[interface(name = "org.freedesktop.UDisks2.Block")]
impl Block {
    #[zbus(property)]
    fn hint_ignore(&self) -> bool {
        self.device.hint_ignore
    }

    #[zbus(property)]
    fn hint_system(&self) -> bool {
        self.device.hint_system
    }

    #[zbus(property)]
    fn hint_name(&self) -> String {
        self.device.hint_name.to_owned()
    }

    #[zbus(property)]
    fn id_label(&self) -> String {
        self.device.label.to_owned()
    }

    #[zbus(property)]
    fn drive(&self) -> OwnedObjectPath {
        let path = match &self.device.drive {
            Some(drive) => format!("{ROOT}/drives/{}", drive.node),
            None => "/".to_owned(),
        };
        OwnedObjectPath::try_from(path).unwrap_or_default()
    }
}

struct Filesystem {
    node: &'static str,
    world: Shared,
}

#[interface(name = "org.freedesktop.UDisks2.Filesystem")]
impl Filesystem {
    async fn mount(
        &self,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<String, Refused> {
        let at = {
            let mut world = locked(&self.world);
            world.calls.push(format!("mount {}", self.node));
            if world.deny {
                return Err(Refused::NotAuthorized("not allowed".to_owned()));
            }
            let at = format!("/run/media/p/{}", self.node);
            world.mounted.insert(self.node, Some(at.clone()));
            at
        };
        self.mount_points_changed(&emitter).await?;
        Ok(at)
    }

    async fn unmount(
        &self,
        _options: HashMap<String, OwnedValue>,
        #[zbus(signal_emitter)] emitter: SignalEmitter<'_>,
    ) -> Result<(), Refused> {
        {
            let mut world = locked(&self.world);
            world.calls.push(format!("unmount {}", self.node));
            if world.busy {
                return Err(Refused::DeviceBusy("busy".to_owned()));
            }
            world.mounted.insert(self.node, None);
        }
        self.mount_points_changed(&emitter).await?;
        Ok(())
    }

    #[zbus(property)]
    fn mount_points(&self) -> Vec<Vec<u8>> {
        let world = locked(&self.world);
        match world.mounted.get(self.node) {
            Some(Some(at)) => {
                let mut bytes = at.clone().into_bytes();
                bytes.push(0);
                vec![bytes]
            }
            Some(None) | None => Vec::new(),
        }
    }
}

struct Drive {
    spec: DriveSpec,
    world: Shared,
}

#[interface(name = "org.freedesktop.UDisks2.Drive")]
impl Drive {
    fn eject(&self, _options: HashMap<String, OwnedValue>) {
        locked(&self.world)
            .calls
            .push(format!("eject {}", self.spec.node));
    }

    fn power_off(&self, _options: HashMap<String, OwnedValue>) {
        locked(&self.world)
            .calls
            .push(format!("power_off {}", self.spec.node));
    }

    #[zbus(property)]
    fn removable(&self) -> bool {
        self.spec.removable
    }

    #[zbus(property)]
    fn ejectable(&self) -> bool {
        self.spec.ejectable
    }

    #[zbus(property)]
    fn can_power_off(&self) -> bool {
        self.spec.can_power_off
    }

    #[zbus(property)]
    fn media_compatibility(&self) -> Vec<String> {
        self.spec.media.iter().map(|m| (*m).to_owned()).collect()
    }
}

/// Puts `device` (and its drive) on the fake service of `connection`.
pub async fn plug(connection: &Connection, world: &Shared, device: &Device) {
    let server = connection.object_server();
    if let Some(spec) = &device.drive {
        let at = format!("{ROOT}/drives/{}", spec.node);
        let drive = Drive {
            spec: spec.clone(),
            world: Arc::clone(world),
        };
        // A second device on the same drive finds it there already.
        let _ = server.at(at, drive).await;
    }
    locked(world)
        .mounted
        .insert(device.node, device.mount.map(str::to_owned));
    let at = format!("{ROOT}/block_devices/{}", device.node);
    server
        .at(
            at.clone(),
            Block {
                device: device.clone(),
            },
        )
        .await
        .unwrap();
    let filesystem = Filesystem {
        node: device.node,
        world: Arc::clone(world),
    };
    server.at(at, filesystem).await.unwrap();
}

/// The fake service, serving `devices` under the name UDisks2 has.
pub async fn serve(connection: &Connection, devices: &[Device]) -> Shared {
    let world: Shared = Arc::default();
    for device in devices {
        plug(connection, &world, device).await;
    }
    connection
        .object_server()
        .at(ROOT, zbus::fdo::ObjectManager)
        .await
        .unwrap();
    connection
        .request_name("org.freedesktop.UDisks2")
        .await
        .unwrap();
    world
}
