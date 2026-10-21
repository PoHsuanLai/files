//! The disk behind traits: listing a folder, following its changes, and remembering how each
//! folder is shown.
//!
//! [`Lister`] turns a folder into [`files_core::Entry`] values and [`Watcher`] reports which names
//! in a folder were added, removed or changed. The real implementations are [`DiskLister`] and
//! [`NotifyWatcher`]; with the `testing` feature, `FakeLister` and `FakeWatcher` stand in for them
//! so the logic above never touches a disk. The crate is runtime-free: it uses plain threads and
//! callbacks, and an async adapter belongs to whoever owns the runtime.
//!
//! ```
//! use files_core::FolderPath;
//! use files_fs::{DiskLister, Lister};
//!
//! let dir = tempfile::tempdir()?;
//! std::fs::write(dir.path().join("a.txt"), "hi")?;
//! let entries = DiskLister.list(&FolderPath::new(dir.path())?)?;
//! assert_eq!(entries.len(), 1);
//! assert_eq!(entries[0].name.display(), "a.txt");
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod error;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod info;
mod list;
mod prefs;
mod watch;

pub use error::{ListError, PrefsError, WatchError};
#[cfg(any(test, feature = "testing"))]
pub use fake::{FakeLister, FakeWatcher};
pub use list::{DiskLister, Lister, Listing, Skipped};
#[cfg(any(test, feature = "testing"))]
pub use prefs::FakePrefs;
pub use prefs::{KEPT, NoPrefs, PrefBook, PrefsStore, TomlPrefs};
pub use watch::{FolderEvent, NotifyWatcher, WatchHandle, Watcher};
