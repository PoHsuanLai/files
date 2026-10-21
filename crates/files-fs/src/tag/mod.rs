//! Tags on the disk: read and written through the [`Tagger`] trait.
//!
//! The real one, [`XattrTagger`], keeps a file's tags in the `user.xdg.tags` extended attribute as
//! a comma-separated list, as GNOME and KDE do, so tags set here show in their file managers and
//! the other way round. [`Tagged`] makes any [`Lister`](crate::Lister) fill in each entry's tags.
//! [`find_tagged`] walks a folder looking for one colour.

mod disk;
#[cfg(any(test, feature = "testing"))]
mod fake;
mod tagged;
mod walk;

use files_core::{ItemRef, Tags};

use crate::error::TagWriteError;

pub use disk::{ATTRIBUTE, XattrTagger};
#[cfg(any(test, feature = "testing"))]
pub use fake::FakeTagger;
pub use tagged::Tagged;
pub use walk::{Flow, Found, WALK_CAP, Walked, find_tagged};

/// Reads and writes the tags of files. Real: [`XattrTagger`]; for tests: `FakeTagger` (feature
/// `testing`).
pub trait Tagger: Send + Sync {
    /// The tags of `item`; none when it has none or they can't be read (a disk with no support for
    /// them has none).
    fn tags(&self, item: &ItemRef) -> Tags;

    /// Gives `item` exactly `tags`; no tags removes the attribute.
    fn set(&self, item: &ItemRef, tags: &Tags) -> Result<(), TagWriteError>;
}

/// A computer that keeps no tags: nothing has any, and none can be added.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoTags;

impl Tagger for NoTags {
    fn tags(&self, _item: &ItemRef) -> Tags {
        Tags::none()
    }

    fn set(&self, _item: &ItemRef, _tags: &Tags) -> Result<(), TagWriteError> {
        Err(TagWriteError::Unsupported)
    }
}
