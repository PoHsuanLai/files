//! Sizes, kinds and dates in the words a person reads. Pure: no clock, no zone, no disk.

mod date;
mod info;
mod kind;
mod ops;
mod size;
mod volumes;

pub use date::date_words;
pub use info::{InfoRow, exact_size_words, info_failure_words, info_rows, items_words, name_rows};
pub use kind::kind_words;
pub use ops::{CANNOT_UNDO, announce_words, busy_words, conflict_words, erase_words};
pub use size::{entry_size_words, size_words};
pub use volumes::volume_trouble_words;

/// What a column shows for something an entry does not have: a folder's size, an unknown date.
pub const NONE: &str = "\u{2014}";
