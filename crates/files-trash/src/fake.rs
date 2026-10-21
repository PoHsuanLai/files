//! A Trash kept in a scratch folder, for tests: it really moves the files, so a test sees them
//! leave and come back, and it never touches the person's own Trash.

use std::fs;
use std::path::{Path, PathBuf};
use std::sync::{Mutex, MutexGuard, PoisonError};
use std::time::{Duration, UNIX_EPOCH};

use files_core::{ByteSize, EntryKind, FileName, ItemRef, Split, TrashId, TrashedItem, keep_both};

use crate::error::TrashError;
use crate::store::Trash;

/// One item held: what it is and where its files are.
#[derive(Debug, Clone)]
struct Held {
    item: TrashedItem,
    stored: PathBuf,
}

#[derive(Debug, Default)]
struct State {
    clock: u64,
    held: Vec<Held>,
}

/// A [`Trash`] whose items live under a folder of the test's choosing.
///
/// The folder must be on the same filesystem as the files the test trashes, as a `TempDir` beside
/// them is. Deletion times count up from one second after the epoch, one per trashed item, so
/// "newest" is the last thrown away.
#[derive(Debug)]
pub struct FakeTrash {
    root: PathBuf,
    state: Mutex<State>,
}

fn locked(state: &Mutex<State>) -> MutexGuard<'_, State> {
    state.lock().unwrap_or_else(PoisonError::into_inner)
}

impl FakeTrash {
    /// A Trash that keeps its items under `root` (made when first needed).
    pub fn new(root: impl Into<PathBuf>) -> FakeTrash {
        FakeTrash {
            root: root.into(),
            state: Mutex::new(State::default()),
        }
    }

    fn files(&self) -> PathBuf {
        self.root.join("files")
    }
}

fn kind_at(path: &Path) -> Result<EntryKind, TrashError> {
    let meta = fs::symlink_metadata(path).map_err(|e| TrashError::of_io(&e))?;
    Ok(if meta.is_dir() {
        EntryKind::Folder
    } else {
        EntryKind::File(ByteSize(meta.len()))
    })
}

fn erase(path: &Path) -> Result<(), TrashError> {
    let meta = fs::symlink_metadata(path).map_err(|e| TrashError::of_io(&e))?;
    let done = if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    done.map_err(|e| TrashError::of_io(&e))
}

impl Trash for FakeTrash {
    fn trash(&self, items: &[ItemRef]) -> Result<Vec<TrashedItem>, TrashError> {
        fs::create_dir_all(self.files()).map_err(|e| TrashError::of_io(&e))?;
        let mut state = locked(&self.state);
        let mut thrown = Vec::new();
        for original in items {
            let kind = kind_at(&original.path())?;
            let taken = |name: &FileName| {
                state
                    .held
                    .iter()
                    .any(|h| h.item.stored_name().as_ref() == Some(name))
            };
            let stored_name = keep_both(&original.name, Split::Extension, taken)
                .map_err(|_| TrashError::Other)?;
            let stored = self.files().join(stored_name.as_os_str());
            fs::rename(original.path(), &stored).map_err(|e| TrashError::of_io(&e))?;
            state.clock += 1;
            let id = self
                .root
                .join("info")
                .join(format!("{}.trashinfo", stored_name.display()));
            let item = TrashedItem::new(
                TrashId::new(id),
                original.clone(),
                UNIX_EPOCH + Duration::from_secs(state.clock),
                kind,
            );
            state.held.push(Held {
                item: item.clone(),
                stored,
            });
            thrown.push(item);
        }
        Ok(thrown)
    }

    fn list(&self) -> Result<Vec<TrashedItem>, TrashError> {
        Ok(locked(&self.state)
            .held
            .iter()
            .map(|held| held.item.clone())
            .collect())
    }

    fn put_back(&self, items: &[TrashedItem]) -> Result<(), TrashError> {
        let mut state = locked(&self.state);
        let held_all = items
            .iter()
            .all(|item| state.held.iter().any(|h| h.item.id == item.id));
        if !held_all {
            return Err(TrashError::NotFound);
        }
        if items
            .iter()
            .any(|item| fs::symlink_metadata(item.original.path()).is_ok())
        {
            return Err(TrashError::Occupied);
        }
        for item in items {
            let Some(at) = state.held.iter().position(|h| h.item.id == item.id) else {
                continue;
            };
            let held = state.held.remove(at);
            fs::create_dir_all(item.original.folder.as_path())
                .map_err(|e| TrashError::of_io(&e))?;
            fs::rename(&held.stored, item.original.path()).map_err(|e| TrashError::of_io(&e))?;
        }
        Ok(())
    }

    fn delete_for_good(&self, items: &[TrashedItem]) -> Result<(), TrashError> {
        let mut state = locked(&self.state);
        for item in items {
            let Some(at) = state.held.iter().position(|h| h.item.id == item.id) else {
                return Err(TrashError::NotFound);
            };
            let held = state.held.remove(at);
            erase(&held.stored)?;
        }
        Ok(())
    }

    fn empty(&self) -> Result<(), TrashError> {
        let mut state = locked(&self.state);
        for held in std::mem::take(&mut state.held) {
            erase(&held.stored)?;
        }
        Ok(())
    }
}
