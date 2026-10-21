//! The real Trash, over the `trash` crate.

use std::path::{Path, PathBuf};
use std::time::{Duration, UNIX_EPOCH};

use files_core::{ByteSize, EntryKind, FileName, FolderPath, ItemRef, TrashId, TrashedItem};
use trash::os_limited;
use trash::{TrashItem, TrashItemSize};

use crate::error::TrashError;
use crate::store::Trash;

/// The person's Trash as the desktop shares it: the home Trash and each volume's own.
///
/// The `trash` crate finds the home Trash from `XDG_DATA_HOME` (or `HOME`) itself; this type
/// reads no environment of its own.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemTrash;

impl Trash for SystemTrash {
    fn trash(&self, items: &[ItemRef]) -> Result<Vec<TrashedItem>, TrashError> {
        let paths: Vec<PathBuf> = items.iter().map(ItemRef::path).collect();
        trash::delete_all(&paths).map_err(TrashError::of_foreign)?;
        let listed = self.list()?;
        Ok(items
            .iter()
            .filter_map(|item| newest(&listed, item))
            .collect())
    }

    fn list(&self) -> Result<Vec<TrashedItem>, TrashError> {
        let listed = os_limited::list().map_err(TrashError::of_foreign)?;
        Ok(listed.iter().filter_map(trashed_of).collect())
    }

    fn put_back(&self, items: &[TrashedItem]) -> Result<(), TrashError> {
        if items
            .iter()
            .any(|item| std::fs::symlink_metadata(item.original.path()).is_ok())
        {
            return Err(TrashError::Occupied);
        }
        os_limited::restore_all(items.iter().map(foreign_of)).map_err(TrashError::of_foreign)
    }

    fn delete_for_good(&self, items: &[TrashedItem]) -> Result<(), TrashError> {
        os_limited::purge_all(items.iter().map(foreign_of)).map_err(TrashError::of_foreign)
    }

    fn empty(&self) -> Result<(), TrashError> {
        let all = os_limited::list().map_err(TrashError::of_foreign)?;
        os_limited::purge_all(all).map_err(TrashError::of_foreign)
    }
}

/// The listed item that `item` became: the newest one whose original path is its path, or the
/// same path with the folder's links resolved (the crate records that one).
fn newest(listed: &[TrashedItem], item: &ItemRef) -> Option<TrashedItem> {
    let direct = item.path();
    let resolved = std::fs::canonicalize(item.folder.as_path())
        .ok()
        .map(|folder| folder.join(item.name.as_os_str()));
    listed
        .iter()
        .filter(|held| {
            let path = held.original.path();
            path == direct || Some(&path) == resolved.as_ref()
        })
        .max_by_key(|held| held.deleted)
        .cloned()
}

/// Our item for the crate's `item`, when its place and name are usable.
fn trashed_of(item: &TrashItem) -> Option<TrashedItem> {
    let original = ItemRef::new(
        FolderPath::new(&item.original_parent).ok()?,
        FileName::new(&item.name).ok()?,
    );
    let secs = u64::try_from(item.time_deleted).unwrap_or(0);
    Some(TrashedItem::new(
        TrashId::new(item.id.clone()),
        original,
        UNIX_EPOCH + Duration::from_secs(secs),
        kind_of(item),
    ))
}

/// A file with its size, or a folder; a file whose size cannot be read counts as empty.
fn kind_of(item: &TrashItem) -> EntryKind {
    match os_limited::metadata(item).map(|meta| meta.size) {
        Ok(TrashItemSize::Entries(_)) => EntryKind::Folder,
        Ok(TrashItemSize::Bytes(bytes)) => EntryKind::File(ByteSize(bytes)),
        Err(_) => EntryKind::File(ByteSize(0)),
    }
}

/// The crate's item for ours.
fn foreign_of(item: &TrashedItem) -> TrashItem {
    TrashItem {
        id: item.id.as_os_str().to_owned(),
        name: item.original.name.as_os_str().to_owned(),
        original_parent: Path::to_path_buf(item.original.folder.as_path()),
        time_deleted: item
            .deleted
            .duration_since(UNIX_EPOCH)
            .ok()
            .and_then(|since| i64::try_from(since.as_secs()).ok())
            .unwrap_or(0),
    }
}
