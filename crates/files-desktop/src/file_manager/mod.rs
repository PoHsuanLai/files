//! `org.freedesktop.FileManager1`: how other programs ask a file manager to show a folder, reveal
//! a file or show its properties ("Show in Folder" in a browser's download list, say).
//!
//! The bus call is parsed into a [`Show`] and handed to a sink the app gives at start; nothing
//! here opens a window.

mod request;
mod server;
mod wire;

pub use request::{Reveal, Show, ShowKind, ShowRequest, StartupId};
pub(crate) use server::{serve, unserve};

use std::sync::Arc;

/// Where the service hands each request: the app's window code.
pub type Sink = Arc<dyn Fn(Show) + Send + Sync>;

/// The object the service lives at.
pub const OBJECT_PATH: &str = "/org/freedesktop/FileManager1";

/// The interface's name, which is also the well-known name a file manager owns.
pub const INTERFACE: &str = "org.freedesktop.FileManager1";
