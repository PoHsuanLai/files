//! The vocabulary of files: what a folder holds, how it is shown and what can be done to it, as
//! plain data.
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

mod conflict;
mod entry;
mod failure;
mod info;
mod item;
mod keep_both;
mod location;
mod mime;
mod name;
mod plan;
mod progress;
mod size;
mod tag;
mod trashed;
mod view;
mod volume;

pub use conflict::{Answer, Conflict, ConflictKind, Reach, Resolution};
pub use entry::{Entry, EntryKind, LinkTo, Special, Visibility};
pub use failure::OpFailure;
pub use info::{Info, ItemCount};
pub use item::{ItemError, ItemRef};
pub use keep_both::{Split, keep_both};
pub use location::{FolderPath, Location, PathError};
pub use mime::{MimeError, MimeType};
pub use name::{FileName, NameError};
pub use plan::{OpKind, OpPlan, Summary, Transfer};
pub use progress::Progress;
pub use size::ByteSize;
pub use tag::{Tag, TagColour, TagError, Tags};
pub use trashed::{TrashId, TrashedItem};
pub use view::{FolderPrefs, HiddenFiles, Sort, SortKey, SortOrder, ViewMode};
pub use volume::{
    Eject, EjectFailure, Mount, MountFailure, ProgramName, Volume, VolumeId, VolumeKind,
    VolumeName, VolumeTrouble,
};
