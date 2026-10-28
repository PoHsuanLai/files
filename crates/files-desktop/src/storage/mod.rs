//! The cloud drives: `org.quire.Storage1`, porter's sync service, as files-core's vocabulary.
//!
//! [`Storage`] is the async side over a bus connection the caller passes in (the session bus, or
//! a private one in a test): the drives, where items stand, and the three asks. [`DesktopClouds`]
//! is the same behind `files_fs::Clouds` for the window's own threads: every call blocks on the
//! runtime it was given. The service may be absent, or go away and come back; a call then fails
//! as `Unavailable`, and the news stream says `Drives` when it comes back so the caller asks
//! again. Nothing here reads the environment, and paths are only ever the ones the service gave.

mod decode;
mod error;
mod follow;

use files_core::{CloudDrive, CloudItem, ItemRef, Refused, SavedCopies};
use files_fs::{CloudError, CloudGuard, Clouds, News};
use porter_dbus::StorageProxy;
use tokio::runtime::Handle;
use tokio::task::JoinHandle;
use zbus::Connection;

pub use error::StorageError;

/// The service's name on the bus: syncd's.
pub const SERVICE: &str = "org.quire.Sync1";

/// The cloud service over a connection.
#[derive(Clone)]
pub struct Storage {
    proxy: StorageProxy<'static>,
}

impl std::fmt::Debug for Storage {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Storage")
    }
}

fn paths(items: &[ItemRef]) -> Vec<String> {
    items
        .iter()
        .map(|item| item.path().to_string_lossy().into_owned())
        .collect()
}

impl Storage {
    /// The service syncd offers, over `connection`. Nothing is called yet, so a service that is
    /// not running is no failure here.
    pub async fn connect(connection: &Connection) -> Result<Storage, StorageError> {
        Storage::connect_to(connection, SERVICE).await
    }

    /// The service under another bus name (a stand-in on a private bus).
    pub async fn connect_to(connection: &Connection, name: &str) -> Result<Storage, StorageError> {
        let proxy = StorageProxy::builder(connection)
            .destination(name.to_owned())?
            .build()
            .await?;
        Ok(Storage { proxy })
    }

    /// The drives the caller may see. A row this version cannot read is left out.
    pub async fn drives(&self) -> Result<Vec<CloudDrive>, StorageError> {
        let rows = self.proxy.drives().await?;
        Ok(rows
            .into_iter()
            .filter_map(|(id, details)| decode::drive(id, &details).ok())
            .collect())
    }

    /// Where each of `items` stands. One the service has no word for, or this version cannot
    /// read, is left out.
    pub async fn items(
        &self,
        items: &[ItemRef],
    ) -> Result<Vec<(ItemRef, CloudItem)>, StorageError> {
        let details = self.proxy.items(paths(items)).await?;
        let mut rows: Vec<(String, _)> = details.into_iter().collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        Ok(rows
            .into_iter()
            .filter_map(|(path, details)| decode::item(&path, &details).ok())
            .collect())
    }

    /// Download Now.
    pub async fn download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, StorageError> {
        Ok(decode::refused(self.proxy.download(paths(items)).await?))
    }

    /// Remove Download.
    pub async fn remove_download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, StorageError> {
        Ok(decode::refused(
            self.proxy.remove_download(paths(items)).await?,
        ))
    }

    /// Save a Copy: each online document as a file next to it.
    pub async fn save_copy(&self, items: &[ItemRef]) -> Result<SavedCopies, StorageError> {
        let (saved, refused) = self.proxy.save_copy(paths(items)).await?;
        Ok(decode::copies(saved, refused))
    }

    /// Keep Downloaded, on (`keep`) or off.
    pub async fn keep(&self, items: &[ItemRef], keep: bool) -> Result<Vec<Refused>, StorageError> {
        let wire = self.proxy.keep(paths(items), keep).await?;
        Ok(decode::refused(wire))
    }

    /// Tells `news` of every change until the task ends: joins the service's watch, hears its
    /// signals, and joins again when the service comes back. Run it on the runtime.
    pub async fn follow(&self, news: News) {
        follow::run(self.proxy.clone(), news).await;
    }
}

/// A task that is stopped when this is dropped.
struct Stops(JoinHandle<()>);

impl Drop for Stops {
    fn drop(&mut self) {
        self.0.abort();
    }
}

/// The cloud service behind [`Clouds`], for threads that are not on the runtime.
#[derive(Debug, Clone)]
pub struct DesktopClouds {
    storage: Storage,
    runtime: Handle,
}

impl DesktopClouds {
    /// The service `storage`, called from threads that are not on `runtime`.
    pub fn new(storage: Storage, runtime: Handle) -> DesktopClouds {
        DesktopClouds { storage, runtime }
    }
}

/// What a person's window learns of a failure: the service is out of reach, or answered in a way
/// this version cannot read.
fn failed(error: StorageError) -> CloudError {
    match error {
        StorageError::Bus(_) => CloudError::Unavailable,
        StorageError::Missing(_) | StorageError::Unknown { .. } | StorageError::Path(_) => {
            CloudError::Unreadable
        }
    }
}

impl Clouds for DesktopClouds {
    fn drives(&self) -> Result<Vec<CloudDrive>, CloudError> {
        self.runtime.block_on(self.storage.drives()).map_err(failed)
    }

    fn items(&self, items: &[ItemRef]) -> Result<Vec<(ItemRef, CloudItem)>, CloudError> {
        self.runtime
            .block_on(self.storage.items(items))
            .map_err(failed)
    }

    fn download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, CloudError> {
        self.runtime
            .block_on(self.storage.download(items))
            .map_err(failed)
    }

    fn remove_download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, CloudError> {
        self.runtime
            .block_on(self.storage.remove_download(items))
            .map_err(failed)
    }

    fn save_copy(&self, items: &[ItemRef]) -> Result<SavedCopies, CloudError> {
        self.runtime
            .block_on(self.storage.save_copy(items))
            .map_err(failed)
    }

    fn keep(&self, items: &[ItemRef], keep: bool) -> Result<Vec<Refused>, CloudError> {
        self.runtime
            .block_on(self.storage.keep(items, keep))
            .map_err(failed)
    }

    fn watch(&self, news: News) -> Result<CloudGuard, CloudError> {
        let storage = self.storage.clone();
        let task = self
            .runtime
            .spawn(async move { storage.follow(news).await });
        Ok(CloudGuard::new(Stops(task)))
    }
}
