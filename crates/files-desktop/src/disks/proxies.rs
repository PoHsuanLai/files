//! The parts of the disk service this crate calls. Properties are read from the managed-objects
//! reply in one call, so these name only the methods.

use std::collections::HashMap;

use zbus::proxy;
use zbus::zvariant::Value;

/// A filesystem on a block device.
#[proxy(
    interface = "org.freedesktop.UDisks2.Filesystem",
    default_service = "org.freedesktop.UDisks2",
    gen_blocking = false
)]
pub(super) trait Filesystem {
    /// Mounts it, giving the folder.
    fn mount(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<String>;

    /// Unmounts it.
    fn unmount(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;

    /// Where it is mounted, as NUL-ended byte strings.
    #[zbus(property)]
    fn mount_points(&self) -> zbus::Result<Vec<Vec<u8>>>;
}

/// A drive, for taking it away after its disks are unmounted.
#[proxy(
    interface = "org.freedesktop.UDisks2.Drive",
    default_service = "org.freedesktop.UDisks2",
    gen_blocking = false
)]
pub(super) trait Drive {
    /// Ejects its media.
    fn eject(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;

    /// Switches it off, so it can be unplugged.
    fn power_off(&self, options: HashMap<&str, Value<'_>>) -> zbus::Result<()>;
}
