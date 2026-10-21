//! The vocabulary of files: what a folder holds and how it is shown, as plain data.
//!
//! Nothing here reads the disk, the clock or the environment, and nothing names a runtime, a bus
//! or a UI. Every other crate of the app speaks these types: `files-fs` fills them from the disk,
//! `files-model` steps them, `files-ui` draws them.
//!
//! ```
//! use files_core::{FileName, NameError};
//! use std::ffi::OsStr;
//!
//! let name = FileName::new("Report.final.pdf")?;
//! assert_eq!(name.stem(), OsStr::new("Report.final"));
//! assert_eq!(name.extension(), Some(OsStr::new("pdf")));
//! assert_eq!(FileName::new("a/b"), Err(NameError::Slash));
//! # Ok::<(), NameError>(())
//! ```

mod name;

pub use name::{FileName, NameError};
