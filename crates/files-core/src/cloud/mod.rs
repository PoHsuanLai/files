//! Cloud drives: the accounts a person turned on for Files, shown as folders, and where each
//! file or folder in them stands (in the cloud, downloading, waiting to upload...).
//!
//! Plain data. The service that knows them is a trait of `files-fs`; the model keeps what it was
//! last told.

mod drive;
mod item;
mod text;
mod url;

pub use drive::{CloudDrive, DriveState, Usage};
pub use item::{CloudItem, CloudKind, CloudState, Refused, Saved, SavedCopies, Share};
pub use text::{AccountLabel, CloudWords, DriveId, ProviderName};
pub use url::{OnlineUrl, UrlError};

use crate::ItemRef;

/// What the service tells without being asked.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CloudNews {
    /// The drives changed (one came or went, a state, the space used): ask for the list again.
    Drives,
    /// Items of one drive changed, each with its new details.
    Items(DriveId, Vec<(ItemRef, CloudItem)>),
}
