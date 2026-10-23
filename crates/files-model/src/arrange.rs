//! Which entries a folder shows, and in what order.

use files_core::{Entry, FolderPrefs, FoldersOnTop, HiddenFiles, SortKey, Visibility};

use crate::open::{Opens, opens};
use crate::sort::compare;

/// The indexes into `entries` of the rows to show, in order: hidden entries are left out unless
/// `hidden` shows them, and the rest are sorted by `prefs.sort`. With `folders` first and a sort by
/// name, the folders come before the files in either direction.
pub fn arrange(
    entries: &[Entry],
    prefs: &FolderPrefs,
    hidden: HiddenFiles,
    folders: FoldersOnTop,
) -> Vec<usize> {
    let mut shown: Vec<usize> = entries
        .iter()
        .enumerate()
        .filter(|(_, e)| match hidden {
            HiddenFiles::Shown => true,
            HiddenFiles::Left => e.visibility() == Visibility::Shown,
        })
        .map(|(at, _)| at)
        .collect();
    shown.sort_by(|a, b| match (entries.get(*a), entries.get(*b)) {
        (Some(x), Some(y)) => compare(x, y, prefs.sort),
        _ => a.cmp(b),
    });
    if folders == FoldersOnTop::First && prefs.sort.key == SortKey::Name {
        // A stable sort on "is not a folder": folders first, each side keeping its order.
        shown.sort_by_key(|at| {
            entries
                .get(*at)
                .is_none_or(|entry| opens(entry) != Opens::Folder)
        });
    }
    shown
}
