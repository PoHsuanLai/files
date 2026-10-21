//! Reading the disk: listing a folder, checking one, the facts of an item and following a folder.

use std::sync::Arc;

use files_core::{FileName, FolderPath, ItemRef, Location};
use files_fs::{FolderEvent, ListError, Lister};
use files_model::{Change, Input, ListFailure, TabId};

use super::{Edge, Heard, ops};

impl Edge {
    /// Finds out off the window's thread whether `folder` can be listed, which is what makes it
    /// somewhere to go.
    pub(super) fn check(&self, tab: TabId, folder: FolderPath) {
        let (heard, lister) = (self.heard.clone(), Arc::clone(&self.lister));
        let failed = folder.clone();
        let started = Edge::work("files-check", move || {
            let result = lister.list(&folder).map(|_| ()).map_err(failure_of);
            let _ = heard.send(Heard::Tab(tab, Input::FolderChecked(folder, result)));
        });
        if started.is_err() {
            self.say(Heard::Tab(
                tab,
                Input::FolderChecked(failed, Err(ListFailure::Other)),
            ));
        }
    }

    pub(super) fn list(&self, tab: TabId, at: Location) {
        let (heard, lister) = (self.heard.clone(), Arc::clone(&self.lister));
        let trash = Arc::clone(&self.seams.trash);
        let failed = at.clone();
        let started = Edge::work("files-list", move || {
            let result = match &at {
                Location::Folder(folder) => lister.list(folder).map_err(failure_of),
                Location::Trash => trash
                    .list()
                    .map(|items| items.into_iter().filter_map(ops::row_of).collect())
                    .map_err(ops::failure_of),
                // A search is asked for with `Effect::Find`, not listed.
                Location::Tag(_) | _ => Err(ListFailure::Other),
            };
            let _ = heard.send(Heard::Tab(tab, Input::Listed(at, result)));
        });
        if started.is_err() {
            self.say(Heard::Tab(
                tab,
                Input::Listed(failed, Err(ListFailure::Other)),
            ));
        }
    }

    /// Reads the facts of `item` for Get Info, off the window's thread.
    pub(super) fn read_info(&self, tab: TabId, item: ItemRef) {
        let (heard, lister) = (self.heard.clone(), Arc::clone(&self.lister));
        let failed = item.clone();
        let started = Edge::work("files-info", move || {
            let result = lister.info(&item).map_err(failure_of);
            let _ = heard.send(Heard::Tab(tab, Input::InfoRead(item, result)));
        });
        if started.is_err() {
            self.say(Heard::Tab(
                tab,
                Input::InfoRead(failed, Err(ListFailure::Other)),
            ));
        }
    }

    pub(super) fn watch(&self, tab: TabId, at: Location) {
        let Location::Folder(folder) = &at else {
            return;
        };
        let sink = {
            let (heard, lister) = (self.heard.clone(), Arc::clone(&self.lister));
            let (place, folder) = (at.clone(), folder.clone());
            move |event: FolderEvent| {
                let change = change_of(&*lister, &folder, event);
                let _ = heard.send(Heard::Tab(tab, Input::Changed(place.clone(), change)));
            }
        };
        match self.seams.watcher.watch(folder, Box::new(sink)) {
            Ok(handle) => {
                self.watches.borrow_mut().insert((tab, at), handle);
            }
            Err(err) => self.say(Heard::Told(err.to_string())),
        }
    }
}

/// The failure a listing error is to the browser.
fn failure_of(err: ListError) -> ListFailure {
    match err {
        ListError::NotFound => ListFailure::NotFound,
        ListError::NoPermission => ListFailure::NoPermission,
        ListError::NotAFolder => ListFailure::NotAFolder,
        ListError::Other(_) => ListFailure::Other,
        // An error files-fs adds later is an ordinary failure until this names it.
        _ => ListFailure::Other,
    }
}

/// The change a watcher's `event` is to the listing of `folder`. A new or changed name is looked
/// up again; one that is gone by then is a removal.
pub fn change_of(lister: &dyn Lister, folder: &FolderPath, event: FolderEvent) -> Change {
    match event {
        FolderEvent::Added(name) => seen(lister, folder, name, Change::Added),
        FolderEvent::Changed(name) => seen(lister, folder, name, Change::Updated),
        FolderEvent::Removed(name) => Change::Removed(name),
        FolderEvent::Rescan | FolderEvent::Gone => Change::Rescan,
        // An event files-fs adds later means the listing may be stale.
        _ => Change::Rescan,
    }
}

/// `name` as the lister sees it now, as `make` says it; a removal if it is gone, a rescan if it
/// can't be read.
fn seen(
    lister: &dyn Lister,
    folder: &FolderPath,
    name: FileName,
    make: fn(files_core::Entry) -> Change,
) -> Change {
    match lister.entry(folder, &name) {
        Ok(entry) => make(entry),
        Err(ListError::NotFound) => Change::Removed(name),
        Err(_) => Change::Rescan,
    }
}
