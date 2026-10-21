//! In-memory stand-ins for the disk, for tests of the crates above.

use std::collections::HashMap;
use std::sync::{Arc, Mutex, MutexGuard, PoisonError};

use files_core::{Entry, FileName, FolderPath};

use crate::error::{ListError, WatchError};
use crate::list::Lister;
use crate::watch::{FolderEvent, WatchHandle, Watcher};

fn locked<T>(mutex: &Mutex<T>) -> MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(PoisonError::into_inner)
}

/// A [`Lister`] that answers from a map; a folder never set is not found.
#[derive(Debug, Default)]
pub struct FakeLister {
    folders: Mutex<HashMap<FolderPath, Result<Vec<Entry>, ListError>>>,
}

impl FakeLister {
    /// A lister that knows no folder.
    pub fn new() -> FakeLister {
        FakeLister::default()
    }

    /// Makes listing `folder` give `result` from now on.
    pub fn set(&self, folder: FolderPath, result: Result<Vec<Entry>, ListError>) {
        locked(&self.folders).insert(folder, result);
    }
}

impl Lister for FakeLister {
    fn list(&self, folder: &FolderPath) -> Result<Vec<Entry>, ListError> {
        locked(&self.folders)
            .get(folder)
            .cloned()
            .unwrap_or(Err(ListError::NotFound))
    }

    fn entry(&self, folder: &FolderPath, name: &FileName) -> Result<Entry, ListError> {
        self.list(folder)?
            .into_iter()
            .find(|entry| &entry.name == name)
            .ok_or(ListError::NotFound)
    }
}

type Sink = Box<dyn Fn(FolderEvent) + Send>;

#[derive(Default)]
struct Registry {
    next: u64,
    sinks: Vec<(u64, FolderPath, Sink)>,
}

/// A [`Watcher`] that records what is watched and lets a test push events to it.
#[derive(Default)]
pub struct FakeWatcher {
    registry: Arc<Mutex<Registry>>,
}

impl std::fmt::Debug for FakeWatcher {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FakeWatcher")
            .field("watched", &self.watched())
            .finish()
    }
}

impl FakeWatcher {
    /// A watcher with nothing watched.
    pub fn new() -> FakeWatcher {
        FakeWatcher::default()
    }

    /// The folders watched now, oldest first (one per live handle).
    pub fn watched(&self) -> Vec<FolderPath> {
        let registry = locked(&self.registry);
        registry.sinks.iter().map(|(_, f, _)| f.clone()).collect()
    }

    /// Delivers `event` to every sink watching `folder`. The sinks run on the calling thread, so
    /// a sink must not call back into this watcher.
    pub fn push(&self, folder: &FolderPath, event: FolderEvent) {
        let registry = locked(&self.registry);
        for (_, watched, sink) in &registry.sinks {
            if watched == folder {
                sink(event.clone());
            }
        }
    }
}

impl Watcher for FakeWatcher {
    fn watch(&self, folder: &FolderPath, sink: Sink) -> Result<WatchHandle, WatchError> {
        let mut registry = locked(&self.registry);
        let id = registry.next;
        registry.next += 1;
        registry.sinks.push((id, folder.clone(), sink));
        Ok(WatchHandle::new(Registration {
            registry: Arc::clone(&self.registry),
            id,
        }))
    }
}

/// Removes a sink when its handle is dropped.
struct Registration {
    registry: Arc<Mutex<Registry>>,
    id: u64,
}

impl Drop for Registration {
    fn drop(&mut self) {
        locked(&self.registry).sinks.retain(|(id, _, _)| *id != self.id);
    }
}
