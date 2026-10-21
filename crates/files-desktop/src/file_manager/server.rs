//! The bus object.

use zbus::{Connection, interface};

use super::request::{Show, ShowKind};
use super::{OBJECT_PATH, Sink};

/// The three calls, each handed to the sink as a [`Show`].
pub(crate) struct FileManager1 {
    sink: Sink,
}

impl FileManager1 {
    fn hand_over(&self, kind: ShowKind, uris: &[String], startup_id: &str) {
        if let Some(show) = Show::from_wire(kind, uris, startup_id) {
            (self.sink)(show);
        }
    }
}

#[interface(name = "org.freedesktop.FileManager1")]
impl FileManager1 {
    /// Open each folder in a window.
    fn show_folders(&self, uris: Vec<String>, startup_id: String) {
        self.hand_over(ShowKind::Folders, &uris, &startup_id);
    }

    /// Open each file's folder with the file selected.
    fn show_items(&self, uris: Vec<String>, startup_id: String) {
        self.hand_over(ShowKind::Items, &uris, &startup_id);
    }

    /// Show each file's information panel.
    fn show_item_properties(&self, uris: Vec<String>, startup_id: String) {
        self.hand_over(ShowKind::Properties, &uris, &startup_id);
    }
}

/// Puts the object on `connection`.
pub(crate) async fn serve(connection: &Connection, sink: Sink) -> zbus::Result<()> {
    connection
        .object_server()
        .at(OBJECT_PATH, FileManager1 { sink })
        .await
        .map(drop)
}

/// Takes it off again.
pub(crate) async fn unserve(connection: &Connection) {
    let _ = connection
        .object_server()
        .remove::<FileManager1, _>(OBJECT_PATH)
        .await;
}
