//! Following one folder's changes.

use std::path::{Path, PathBuf};

use files_core::{FileName, FolderPath};
use notify::event::{ModifyKind, RenameMode};
use notify::{ErrorKind, Event, EventKind, RecursiveMode, Watcher as _};

use crate::error::{ListError, WatchError};

/// Something that happened in a watched folder. Only names cross: the caller looks an entry up
/// again with [`crate::Lister::entry`]. Events come as the system gives them, not merged.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum FolderEvent {
    /// A name appeared (created, or moved in).
    Added(FileName),
    /// A name went away (deleted, or moved out).
    Removed(FileName),
    /// A name's contents or details changed.
    Changed(FileName),
    /// Events were lost or unclear: read the folder again.
    Rescan,
    /// The watched folder itself was removed or moved away.
    Gone,
}

/// Follows folders. Real: [`NotifyWatcher`]; for tests: `FakeWatcher` (feature `testing`).
pub trait Watcher {
    /// Calls `sink` for each change among the direct children of `folder`, from another thread,
    /// until the returned handle is dropped.
    fn watch(
        &self,
        folder: &FolderPath,
        sink: Box<dyn Fn(FolderEvent) + Send>,
    ) -> Result<WatchHandle, WatchError>;
}

/// Keeps a watch alive; dropping it stops the watching.
pub struct WatchHandle {
    _guard: Box<dyn Send>,
}

impl WatchHandle {
    /// A handle that stops its watch by dropping `guard`.
    pub fn new(guard: impl Send + 'static) -> WatchHandle {
        WatchHandle {
            _guard: Box::new(guard),
        }
    }
}

impl std::fmt::Debug for WatchHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("WatchHandle")
    }
}

/// The real thing, over the system's file notifications (inotify on Linux).
#[derive(Debug, Clone, Copy, Default)]
pub struct NotifyWatcher;

impl Watcher for NotifyWatcher {
    fn watch(
        &self,
        folder: &FolderPath,
        sink: Box<dyn Fn(FolderEvent) + Send>,
    ) -> Result<WatchHandle, WatchError> {
        let root = folder.as_path().to_path_buf();
        let watched = root.clone();
        let mut watcher = notify::recommended_watcher(move |result: notify::Result<Event>| {
            let events = match result {
                Ok(event) => translate(&watched, event),
                Err(_) => vec![FolderEvent::Rescan],
            };
            events.into_iter().for_each(&sink);
        })
        .map_err(watch_error)?;
        watcher
            .watch(&root, RecursiveMode::NonRecursive)
            .map_err(watch_error)?;
        Ok(WatchHandle::new(watcher))
    }
}

fn watch_error(err: notify::Error) -> WatchError {
    match err.kind {
        ErrorKind::PathNotFound => WatchError::NotFound,
        ErrorKind::MaxFilesWatch => WatchError::TooMany,
        ErrorKind::Io(io) => match ListError::from_io(&io) {
            ListError::NotFound => WatchError::NotFound,
            ListError::NoPermission => WatchError::NoPermission,
            ListError::NotAFolder | ListError::Other(_) => WatchError::Other,
        },
        ErrorKind::Generic(_) | ErrorKind::WatchNotFound | ErrorKind::InvalidConfig(_) => {
            WatchError::Other
        }
    }
}

/// The folder events one system event stands for. Paths that are not direct children of
/// `folder` are ignored, except the folder itself going away.
fn translate(folder: &Path, event: Event) -> Vec<FolderEvent> {
    if event.need_rescan() {
        return vec![FolderEvent::Rescan];
    }
    let paths = &event.paths;
    let itself = paths.iter().any(|p| p == folder);
    match event.kind {
        EventKind::Create(_) => each(folder, paths, FolderEvent::Added),
        EventKind::Remove(_) | EventKind::Modify(ModifyKind::Name(RenameMode::From)) => {
            if itself {
                vec![FolderEvent::Gone]
            } else {
                each(folder, paths, FolderEvent::Removed)
            }
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::To)) => {
            each(folder, paths, FolderEvent::Added)
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Both)) => {
            let mut events = each(folder, paths.get(..1).unwrap_or(&[]), FolderEvent::Removed);
            events.extend(each(
                folder,
                paths.get(1..).unwrap_or(&[]),
                FolderEvent::Added,
            ));
            events
        }
        EventKind::Modify(ModifyKind::Name(RenameMode::Any | RenameMode::Other)) => {
            if each(folder, paths, FolderEvent::Changed).is_empty() {
                Vec::new()
            } else {
                vec![FolderEvent::Rescan]
            }
        }
        EventKind::Modify(
            ModifyKind::Any | ModifyKind::Data(_) | ModifyKind::Metadata(_) | ModifyKind::Other,
        ) => each(folder, paths, FolderEvent::Changed),
        EventKind::Any | EventKind::Access(_) | EventKind::Other => Vec::new(),
    }
}

/// `make` of the name of each path that is a direct child of `folder`.
fn each(folder: &Path, paths: &[PathBuf], make: fn(FileName) -> FolderEvent) -> Vec<FolderEvent> {
    paths
        .iter()
        .filter(|path| path.parent() == Some(folder))
        .filter_map(|path| FileName::new(path.file_name()?).ok())
        .map(make)
        .collect()
}
