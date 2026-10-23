//! The freedesktop.org Trash behind a trait.
//!
//! [`Trash`] moves items to the Trash and hands back a [`TrashedItem`](files_core::TrashedItem)
//! for each, lists what the Trash holds (every volume's `.Trash-$uid` folder as well as the home
//! one), puts items back where they came from, erases them for good and empties the Trash. The
//! real one is [`SystemTrash`], over the `trash` crate; with the `testing` feature, `FakeTrash`
//! keeps a Trash in a scratch folder so tests never touch the person's own.
//!
//! ```
//! use files_core::{FileName, FolderPath, ItemRef};
//! use files_trash::{FakeTrash, Trash};
//!
//! let scratch = tempfile::tempdir()?;
//! let (home, bin) = (scratch.path().join("home"), scratch.path().join("bin"));
//! std::fs::create_dir(&home)?;
//! std::fs::write(home.join("a.txt"), "hi")?;
//!
//! let trash = FakeTrash::new(bin);
//! let item = ItemRef::new(FolderPath::new(&home)?, FileName::new("a.txt")?);
//! let thrown = trash.trash(&[item.clone()])?;
//! assert!(!item.path().exists());
//! assert_eq!(trash.list()?, thrown);
//!
//! trash.put_back(&thrown)?;
//! assert!(item.path().exists());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod error;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod retention;
mod store;
mod system;

pub use error::TrashError;
#[cfg(any(test, feature = "testing"))]
pub use fake::FakeTrash;
pub use retention::{THIRTY_DAYS, expired, sweep};
pub use store::Trash;
pub use system::SystemTrash;
