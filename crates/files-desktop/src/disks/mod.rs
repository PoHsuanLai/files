//! The computer's disks from UDisks2 on the system bus: which are attached, opening one, ejecting
//! one, and a stream that says when the set changes.
//!
//! Nothing here reads the environment. The caller passes the connection (the system bus, or a
//! private one in a test) and a [`DiskUsers`] that names the programs holding a disk open.
//! Authorization is the service's: it asks polkit, and a prompt is the person's desktop's to show.

mod act;
mod facts;
mod proxies;
mod users;

use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::Arc;

use files_core::Volume;
use futures_util::StreamExt;
use zbus::fdo::{ManagedObjects, ObjectManagerProxy};
use zbus::message::Type;
use zbus::zvariant::{OwnedObjectPath, OwnedValue};
use zbus::{Connection, MatchRule, MessageStream};

use crate::error::DiskError;
use facts::{BlockFacts, DriveFacts, volume_of};

#[cfg(feature = "testing")]
pub use users::FakeUsers;
pub use users::{DiskUsers, NoUsers, ProcUsers};

/// The service's name on the bus.
pub const SERVICE: &str = "org.freedesktop.UDisks2";
/// Where its objects begin.
pub const ROOT: &str = "/org/freedesktop/UDisks2";

const FILESYSTEM: &str = "org.freedesktop.UDisks2.Filesystem";
const BLOCK: &str = "org.freedesktop.UDisks2.Block";
const DRIVE: &str = "org.freedesktop.UDisks2.Drive";

/// The computer's disks.
#[derive(Clone)]
pub struct Disks {
    connection: Connection,
    users: Arc<dyn DiskUsers>,
}

impl std::fmt::Debug for Disks {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Disks")
    }
}

/// A stream of "the disks may have changed", ended when the connection closes.
pub struct Changes {
    messages: MessageStream,
}

impl std::fmt::Debug for Changes {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Changes")
    }
}

impl Changes {
    /// Waits for the next change; `None` once the connection is gone.
    pub async fn next(&mut self) -> Option<()> {
        loop {
            match self.messages.next().await? {
                Ok(_) => return Some(()),
                // A message that could not be read says nothing about the disks.
                Err(_) => continue,
            }
        }
    }
}

impl Disks {
    /// Disks over `connection`, finding who holds a disk open with `users`.
    pub fn new(connection: Connection, users: Arc<dyn DiskUsers>) -> Disks {
        Disks { connection, users }
    }

    /// The disks over the system bus.
    pub async fn system(users: Arc<dyn DiskUsers>) -> Result<Disks, DiskError> {
        Ok(Disks::new(Connection::system().await?, users))
    }

    /// The disks to show, in the sidebar's order: external drives first, then others, by name.
    pub async fn list(&self) -> Result<Vec<Volume>, DiskError> {
        let objects = self.objects().await?;
        let mut volumes: Vec<Volume> = objects
            .iter()
            .filter_map(|(path, interfaces)| block_of(&objects, path, interfaces))
            .filter_map(|block| volume_of(&block))
            .collect();
        volumes.sort_by_key(|volume| {
            (
                volume.kind,
                volume.name.as_str().to_owned(),
                volume.id.as_str().to_owned(),
            )
        });
        Ok(volumes)
    }

    /// A stream that yields whenever a disk came, went, was mounted or unmounted. Subscribed
    /// before it returns, so nothing between a [`list`](Disks::list) that follows and the first
    /// wait is missed.
    pub async fn changes(&self) -> Result<Changes, DiskError> {
        let rule = MatchRule::builder()
            .msg_type(Type::Signal)
            .path_namespace(ROOT)?
            .build();
        let messages = MessageStream::for_match_rule(rule, &self.connection, None).await?;
        Ok(Changes { messages })
    }

    async fn objects(&self) -> Result<ManagedObjects, DiskError> {
        let manager = ObjectManagerProxy::builder(&self.connection)
            .destination(SERVICE)?
            .path(ROOT)?
            .build()
            .await?;
        Ok(manager
            .get_managed_objects()
            .await
            .map_err(zbus::Error::from)?)
    }
}

/// The facts of the block device at `path`, when it holds a filesystem.
fn block_of(
    objects: &ManagedObjects,
    path: &OwnedObjectPath,
    interfaces: &HashMap<zbus::names::OwnedInterfaceName, HashMap<String, OwnedValue>>,
) -> Option<BlockFacts> {
    let props = |name: &str| {
        interfaces
            .iter()
            .find(|(interface, _)| interface.as_str() == name)
            .map(|(_, props)| props)
    };
    let filesystem = props(FILESYSTEM)?;
    let block = props(BLOCK)?;
    let drive = text_path(block, "Drive")
        .and_then(|drive| objects.iter().find(|(at, _)| at.as_str() == drive))
        .and_then(|(_, interfaces)| {
            interfaces
                .iter()
                .find(|(interface, _)| interface.as_str() == DRIVE)
                .map(|(_, props)| drive_of(props))
        });
    Some(BlockFacts {
        path: path.as_str().to_owned(),
        hint_ignore: flag(block, "HintIgnore"),
        hint_system: flag(block, "HintSystem"),
        hint_name: text(block, "HintName"),
        label: text(block, "IdLabel"),
        mounts: mount_points(filesystem),
        drive,
    })
}

fn drive_of(props: &HashMap<String, OwnedValue>) -> DriveFacts {
    DriveFacts {
        removable: flag(props, "Removable"),
        ejectable: flag(props, "Ejectable"),
        can_power_off: flag(props, "CanPowerOff"),
        media: texts(props, "MediaCompatibility"),
    }
}

fn flag(props: &HashMap<String, OwnedValue>, key: &str) -> bool {
    props
        .get(key)
        .and_then(|value| bool::try_from(value).ok())
        .unwrap_or(false)
}

fn text(props: &HashMap<String, OwnedValue>, key: &str) -> String {
    props
        .get(key)
        .and_then(|value| <&str>::try_from(value).ok())
        .map(str::to_owned)
        .unwrap_or_default()
}

fn text_path(props: &HashMap<String, OwnedValue>, key: &str) -> Option<String> {
    let value = props.get(key)?.try_clone().ok()?;
    let path = OwnedObjectPath::try_from(value).ok()?;
    (path.as_str() != "/").then(|| path.as_str().to_owned())
}

fn texts(props: &HashMap<String, OwnedValue>, key: &str) -> Vec<String> {
    props
        .get(key)
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::<String>::try_from(value).ok())
        .unwrap_or_default()
}

/// The mount points, which the service gives as NUL-ended bytes.
fn mount_points(filesystem: &HashMap<String, OwnedValue>) -> Vec<PathBuf> {
    use std::os::unix::ffi::OsStringExt;
    filesystem
        .get("MountPoints")
        .and_then(|value| value.try_clone().ok())
        .and_then(|value| Vec::<Vec<u8>>::try_from(value).ok())
        .unwrap_or_default()
        .into_iter()
        .map(|mut bytes| {
            while bytes.last() == Some(&0) {
                bytes.pop();
            }
            PathBuf::from(std::ffi::OsString::from_vec(bytes))
        })
        .collect()
}
