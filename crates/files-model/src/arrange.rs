//! Which entries a folder shows, and in what order.

use files_core::{Behaviour, Entry, FolderPrefs, SortKey};

use crate::open::{Opens, opens};
use crate::sort::compare;

/// The indexes into `entries` of the rows to show, in order: hidden entries are left out unless
/// `show_hidden` shows them, and the rest are sorted by `prefs.sort`. With `behaviour` putting
/// folders first and a sort by name, the folders come before the files in either direction.
pub fn arrange(
    entries: &[Entry],
    prefs: &FolderPrefs,
    show_hidden: bool,
    behaviour: Behaviour,
) -> Vec<usize> {
    let mut shown: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| show_hidden || !e.is_hidden())
        .map(|(at, _)| at)
        .collect();
    shown.sort_by(|a, b| match (entries.get(*a), entries.get(*b)) {
        (Some(x), Some(y)) => compare(x, y, prefs.sort),
        _ => a.cmp(b),
    });
    if behaviour.folders_first && prefs.sort.key == SortKey::Name {
        // A stable sort on "is not a folder": folders first, each side keeping its order.
        shown.sort_by_key(|at| {
            entries
                .get(*at)
                .is_none_or(|entry| opens(entry) != Opens::Folder)
        });
    }
    shown
}
