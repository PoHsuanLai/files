//! Items that have been in the Trash long enough to go.

use std::time::{Duration, SystemTime};

use files_core::TrashedItem;

use crate::error::TrashError;
use crate::store::Trash;

/// Finder's "Remove items from the Trash after 30 days".
pub const THIRTY_DAYS: Duration = Duration::from_secs(30 * 24 * 60 * 60);

/// The items of `items` thrown away more than `keep` before `now`. An item with a date after
/// `now` (a clock that moved back) is not old.
pub fn expired(items: Vec<TrashedItem>, now: SystemTime, keep: Duration) -> Vec<TrashedItem> {
    items
        .into_iter()
        .filter(|item| now.duration_since(item.deleted).is_ok_and(|age| age > keep))
        .collect()
}

/// Erases for good the items that have been in `trash` more than `keep` before `now`, and says how
/// many went.
pub fn sweep(trash: &dyn Trash, now: SystemTime, keep: Duration) -> Result<usize, TrashError> {
    let old = expired(trash.list()?, now, keep);
    if old.is_empty() {
        return Ok(0);
    }
    trash.delete_for_good(&old)?;
    Ok(old.len())
}
