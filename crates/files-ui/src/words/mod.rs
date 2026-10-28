//! Sizes, kinds, dates, and the titles of menus, columns and places, in the words a person reads.
//! Pure: no clock, no zone, no disk.

mod cloud;
mod columns;
mod date;
mod info;
mod kind;
mod marks;
mod menu;
mod ops;
mod pane;
mod places;
mod size;
mod volumes;

pub use cloud::{
    SIGN_IN_BUTTON, drive_rows, drive_state_words, item_rows, item_state_words, status_words,
    trouble_title,
};
pub use columns::{column_title, column_title_in};
pub use date::date_words;
pub use info::{InfoRow, exact_size_words, info_failure_words, info_rows, items_words, name_rows};
pub use kind::kind_words;
pub use marks::{
    CONFLICT, DOWNLOADING, DRIVE_FAILED, FAILED, IN_THE_CLOUD, NEEDS_SIGN_IN, OFFLINE, ONLINE,
    PAUSED, SYNCING, UPLOADING, WAITING,
};
pub use menu::{OTHER_APP, command_title, default_app_title, view_title};
pub use ops::{
    CANNOT_UNDO, ExtensionWords, announce_words, busy_words, conflict_words, erase_words,
    extension_words,
};
pub use pane::{many_words, pane_rows};
pub use places::{heading_label, place_label};
pub use size::{entry_size_words, size_words};
pub use volumes::{force_eject_words, volume_trouble_words};

/// What a column shows for something an entry does not have: a folder's size, an unknown date.
pub const NONE: &str = "\u{2014}";
