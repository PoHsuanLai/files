//! Which entries a folder shows, and in what order.

use files_core::{Entry, FolderPrefs, HiddenFiles, Visibility};

use crate::sort::compare;

/// The indexes into `entries` of the rows to show, in order: hidden entries are left out unless
/// `hidden` shows them, and the rest are sorted by `prefs.sort`.
pub fn arrange(entries: &[Entry], prefs: &FolderPrefs, hidden: HiddenFiles) -> Vec<usize> {
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
    shown
}
