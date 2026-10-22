//! What a tile asks a thumbnail of, and what it is given.

use std::time::SystemTime;

use ds::prelude::ImageSource;
use files_core::{Entry, EntryKind, ItemRef, LinkTo};

/// The version of a file a thumbnail was asked for: a file that changes on disk is a new version,
/// and a thumbnail of the old one is not shown for it.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Version {
    /// Its size in bytes.
    pub len: u64,
    /// When its contents last changed; `None` when the filesystem does not say.
    pub modified: Option<SystemTime>,
}

/// One thumbnail asked for: an item, as it was when the folder was listed.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ThumbKey {
    /// The item.
    pub item: ItemRef,
    /// Its version.
    pub version: Version,
}

impl ThumbKey {
    /// The key of `entry`, which stands for `item`; `None` for anything that is not a file (a
    /// folder keeps its icon, and so do the things that are neither).
    pub fn of(item: ItemRef, entry: &Entry) -> Option<ThumbKey> {
        let len = match &entry.kind {
            EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) => size.0,
            EntryKind::Folder | EntryKind::Link(_) | EntryKind::Special(_) => return None,
            // A kind files-core adds later has no thumbnail until this names it.
            _ => return None,
        };
        Some(ThumbKey {
            item,
            version: Version {
                len,
                modified: entry.modified,
            },
        })
    }
}

/// What came of asking for a thumbnail.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Thumbed {
    /// A small picture of the file, to draw in place of its icon.
    Picture(ImageSource),
    /// The file has none (it is of a kind nothing pictures, or it can't be read): its icon stays.
    Icon,
}
