//! A file or folder of a cloud drive: where it stands.

use crate::{CloudWords, ItemRef, OnlineUrl};

/// Where one file or folder stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CloudState {
    /// Listed, not downloaded.
    Cloud,
    /// Downloading.
    Downloading,
    /// Downloaded and the same as the drive's copy.
    Downloaded,
    /// Changed here and being sent.
    Uploading,
    /// Changed here and not yet sent.
    Waiting,
    /// It could not be downloaded or sent.
    Error,
    /// Changed here and on the drive both; both versions are kept.
    Conflict,
}

/// Whether the person keeps an item downloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Keeping {
    /// Kept: never removed to free space.
    Kept,
    /// Not kept.
    NotKept,
}

/// What an item is.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum CloudKind {
    /// A file with content.
    File,
    /// A folder.
    Folder,
    /// A document that lives only online: opening it opens its address in the browser.
    Online,
}

/// How far a transfer is, in thousandths.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Default)]
pub struct Share(u16);

impl Share {
    /// The share `ratio` (0 to 1) stands for; out of range is clamped, not a number is nothing.
    pub fn from_ratio(ratio: f64) -> Share {
        if ratio.is_nan() {
            return Share(0);
        }
        Share((ratio.clamp(0.0, 1.0) * 1000.0).round() as u16)
    }

    /// The share in thousandths, 0 to 1000.
    pub fn permille(self) -> u16 {
        self.0
    }
}

/// One item's cloud details, as the service gives them.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CloudItem {
    /// Where it stands.
    pub state: CloudState,
    /// Whether it is kept downloaded.
    pub keeping: Keeping,
    /// What it is.
    pub kind: CloudKind,
    /// How far a transfer is, while one moves.
    pub share: Option<Share>,
    /// What went wrong, in a person's words.
    pub words: Option<CloudWords>,
    /// Where an online document opens.
    pub url: Option<OnlineUrl>,
    /// The other version, beside this one, when in conflict.
    pub other: Option<ItemRef>,
}

impl CloudItem {
    /// An item of `kind` in `state`, not kept, with nothing else known.
    pub fn new(kind: CloudKind, state: CloudState) -> CloudItem {
        CloudItem {
            state,
            keeping: Keeping::NotKept,
            kind,
            share: None,
            words: None,
            url: None,
            other: None,
        }
    }

    /// The same item, kept downloaded.
    pub fn kept(self) -> CloudItem {
        CloudItem {
            keeping: Keeping::Kept,
            ..self
        }
    }

    /// The same item, `share` of the way through its transfer.
    pub fn moving(self, share: Share) -> CloudItem {
        CloudItem {
            share: Some(share),
            ..self
        }
    }

    /// The same item, failed for the reason `words`.
    pub fn failed(self, words: CloudWords) -> CloudItem {
        CloudItem {
            words: Some(words),
            ..self
        }
    }

    /// The same item, opening at `url`.
    pub fn opening_at(self, url: OnlineUrl) -> CloudItem {
        CloudItem {
            url: Some(url),
            ..self
        }
    }

    /// The same item, in conflict with the version at `other`.
    pub fn beside(self, other: ItemRef) -> CloudItem {
        CloudItem {
            other: Some(other),
            ..self
        }
    }
}

/// A path an ask did not take, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Refused {
    /// The item.
    pub item: ItemRef,
    /// Why, in a person's words.
    pub words: CloudWords,
}
