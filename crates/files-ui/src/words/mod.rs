//! Sizes, kinds and dates in the words a person reads. Pure: no clock, no zone, no disk.

mod date;
mod info;
mod kind;
mod ops;
mod pane;
mod size;
mod volumes;

pub use date::date_words;
pub use info::{InfoRow, exact_size_words, info_failure_words, info_rows, items_words, name_rows};
pub use kind::kind_words;
pub use ops::{
    CANNOT_UNDO, ExtensionWords, announce_words, busy_words, conflict_words, erase_words,
    extension_words,
};
pub use pane::{many_words, pane_rows};
pub use size::{entry_size_words, size_words};
pub use volumes::{force_eject_words, volume_trouble_words};

/// What a column shows for something an entry does not have: a folder's size, an unknown date.
pub const NONE: &str = "\u{2014}";
