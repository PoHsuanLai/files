//! The cloud drives and online documents, over the session bus: the desktop crate's storage
//! service behind the window's `Clouds` seam, and the portal that opens a document's address.

use std::sync::Arc;

use files_core::OnlineUrl;
use files_desktop::{Bus, DesktopClouds, Storage, open_url, session};
use files_ui::OpenFailure;
use tokio::runtime::Handle;

/// Opens an online document's address in the person's browser, through the desktop portal.
#[derive(Clone)]
pub struct Web {
    connection: Bus,
    runtime: Handle,
}

impl std::fmt::Debug for Web {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Web")
    }
}

impl Web {
    /// Asks the desktop to open `url`. The window's thread must not block on the bus, so the
    /// call is made on the runtime; a failure is only said on stderr, as the other bus work is.
    pub fn open(&self, url: &OnlineUrl) -> Result<(), OpenFailure> {
        let connection = self.connection.clone();
        let url = url.clone();
        self.runtime.spawn(async move {
            if let Err(said) = open_url(&connection, &url).await {
                eprintln!("files: {said}");
            }
        });
        Ok(())
    }
}

/// What the session bus gives the window: the cloud drives, and the browser for their documents.
pub struct Cloud {
    /// The storage service behind the window's seam.
    pub clouds: Arc<DesktopClouds>,
    /// Opens online documents.
    pub web: Web,
}

/// Connects to the session bus for the cloud drives. Without it the window shows none; a
/// storage service that is not running is no failure, the drives appear when it starts.
pub async fn connect(runtime: Handle) -> Option<Cloud> {
    let connection = session().await.ok()?;
    let storage = Storage::connect(&connection).await.ok()?;
    Some(Cloud {
        clouds: Arc::new(DesktopClouds::new(storage, runtime.clone())),
        web: Web {
            connection,
            runtime,
        },
    })
}
