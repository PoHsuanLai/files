//! Get Info for the real disk.

use std::fs;

use files_core::{EntryKind, Info, ItemCount, ItemRef, LinkTo};

use crate::error::ListError;
use crate::list::kind_of;

/// The facts of `item` as the disk has them. A link is described, not followed; a folder (or a
/// link to one) is counted, hidden items left out.
pub(crate) fn describe(item: &ItemRef) -> Result<Info, ListError> {
    let path = item.path();
    let meta = fs::symlink_metadata(&path).map_err(|e| ListError::from_io(&e))?;
    let kind = kind_of(&path, meta.file_type(), files_core::ByteSize(meta.len()));
    let counted = matches!(kind, EntryKind::Folder | EntryKind::Link(LinkTo::Folder));
    let mut info = Info::new(item.clone(), kind);
    if let Ok(at) = meta.created() {
        info = info.created_at(at);
    }
    if let Ok(at) = meta.modified() {
        info = info.modified_at(at);
    }
    if counted && let Some(count) = count_in(&path) {
        info = info.holding(count);
    }
    Ok(info)
}

/// How many entries of the folder at `path` a person sees (names starting with a dot are left
/// out); `None` when it can't be read.
fn count_in(path: &std::path::Path) -> Option<ItemCount> {
    let shown = fs::read_dir(path)
        .ok()?
        .filter_map(Result::ok)
        .filter(|child| !child.file_name().to_string_lossy().starts_with('.'))
        .count();
    Some(ItemCount(u32::try_from(shown).unwrap_or(u32::MAX)))
}
