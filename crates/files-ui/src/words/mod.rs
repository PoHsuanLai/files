//! Sizes, kinds and dates in the words a person reads. Pure: no clock, no zone, no disk.

mod date;
mod kind;
mod size;

pub use date::date_words;
pub use kind::kind_words;
pub use size::{entry_size_words, size_words};

/// What a column shows for something an entry does not have: a folder's size, an unknown date.
pub const NONE: &str = "\u{2014}";
