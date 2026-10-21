//! The window's edges: what carries out the model's [`Effect`]s against the disk and the desktop,
//! and what the window reads from outside (the clock).
//!
//! [`Edge::run`] takes the effects a step returned. Listing, watching and renaming run on threads
//! of their own and answer on a channel as [`Heard`]; the window's loop (`state`) feeds them back
//! to the browser. Everything ambient is read here and in [`Zone`], once, and handed down as values.
//!
//! Not built yet: persisting a folder's view (`Effect::LoadPrefs` answers the defaults and
//! `Effect::SavePrefs` is dropped until the prefs lane lands) and the Trash listing (its
//! sidebar row is left out, so nothing asks for it).

mod opener;
mod rename;
mod zone;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use ds::base::time::clock;
use files_core::{FileName, FolderPath, FolderPrefs, Location};
use files_fs::{DiskLister, FolderEvent, ListError, Lister, NotifyWatcher, WatchHandle, Watcher};
use files_model::{Change, Effect, Input, ListFailure, Now, OpenTarget};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub use opener::Opener;
pub use rename::{DiskRenamer, RenameError, Renamer};
pub use zone::Zone;

/// The window's seams to the outside, each behind a trait so a test gives its own.
#[derive(Clone)]
pub struct Seams {
    /// Reads folders.
    pub lister: Arc<dyn Lister>,
    /// Follows folders.
    pub watcher: Arc<dyn Watcher + Send + Sync>,
    /// Hands files to the apps that open them.
    pub opener: Arc<dyn Opener>,
    /// Renames files.
    pub renamer: Arc<dyn Renamer>,
}

impl Seams {
    /// The real disk, with `opener` to open files.
    pub fn disk(opener: Arc<dyn Opener>) -> Seams {
        Seams {
            lister: Arc::new(DiskLister),
            watcher: Arc::new(NotifyWatcher),
            opener,
            renamer: Arc::new(DiskRenamer),
        }
    }
}

impl fmt::Debug for Seams {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Seams")
    }
}

/// What a worker tells the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Heard {
    /// An answer for the browser.
    Input(Input),
    /// Something the person should read, in plain words.
    Told(String),
}

/// Carries out effects. Lives on the window's thread; its workers answer on the channel it made.
pub struct Edge {
    seams: Seams,
    heard: UnboundedSender<Heard>,
    watches: RefCell<HashMap<Location, WatchHandle>>,
}

impl fmt::Debug for Edge {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("Edge")
            .field("watching", &self.watches.borrow().len())
            .finish_non_exhaustive()
    }
}

impl Edge {
    /// An edge over `seams`, and the end where its answers arrive.
    pub fn new(seams: Seams) -> (Edge, UnboundedReceiver<Heard>) {
        let (heard, inbox) = unbounded_channel();
        let edge = Edge {
            seams,
            heard,
            watches: RefCell::new(HashMap::new()),
        };
        (edge, inbox)
    }

    /// Carries out `effects`, in order. Work that takes time runs on a thread.
    pub fn run(&self, effects: Vec<Effect>) {
        for effect in effects {
            self.carry_out(effect);
        }
    }

    fn carry_out(&self, effect: Effect) {
        match effect {
            Effect::List(at) => self.list(at),
            Effect::Watch(at) => self.watch(at),
            Effect::Unwatch(at) => {
                self.watches.borrow_mut().remove(&at);
            }
            Effect::OpenFiles(targets) => {
                let paths = targets.iter().map(OpenTarget::path).collect();
                self.seams.opener.open(paths);
            }
            Effect::Rename { folder, from, to } => self.rename(folder, from, to),
            // Folders are not remembered yet: every folder opens with the defaults.
            Effect::LoadPrefs(at) => {
                self.say(Heard::Input(Input::PrefsLoaded(at, FolderPrefs::default())));
            }
            Effect::SavePrefs(..) => {}
            // An effect files-model adds later is dropped until this names it.
            _ => {}
        }
    }

    fn say(&self, heard: Heard) {
        // A closed channel means the window is gone.
        let _ = self.heard.send(heard);
    }

    /// Runs `job` on a thread named `name`; an error when the system would not start one.
    fn work(name: &str, job: impl FnOnce() + Send + 'static) -> std::io::Result<()> {
        std::thread::Builder::new()
            .name(name.to_owned())
            .spawn(job)
            .map(|_| ())
    }

    fn list(&self, at: Location) {
        let (heard, lister) = (self.heard.clone(), Arc::clone(&self.seams.lister));
        let failed = at.clone();
        let started = Edge::work("files-list", move || {
            let result = match &at {
                Location::Folder(folder) => lister.list(folder).map_err(failure_of),
                // The Trash is not listed yet; nothing asks for it.
                Location::Trash => Err(ListFailure::Other),
                _ => Err(ListFailure::Other),
            };
            let _ = heard.send(Heard::Input(Input::Listed(at, result)));
        });
        if started.is_err() {
            self.say(Heard::Input(Input::Listed(failed, Err(ListFailure::Other))));
        }
    }

    fn watch(&self, at: Location) {
        let Location::Folder(folder) = &at else {
            return;
        };
        let sink = {
            let (heard, lister) = (self.heard.clone(), Arc::clone(&self.seams.lister));
            let (place, folder) = (at.clone(), folder.clone());
            move |event: FolderEvent| {
                let change = change_of(&*lister, &folder, event);
                let _ = heard.send(Heard::Input(Input::Changed(place.clone(), change)));
            }
        };
        match self.seams.watcher.watch(folder, Box::new(sink)) {
            Ok(handle) => {
                self.watches.borrow_mut().insert(at, handle);
            }
            Err(err) => self.say(Heard::Told(err.to_string())),
        }
    }

    fn rename(&self, folder: FolderPath, from: FileName, to: FileName) {
        let (heard, renamer) = (self.heard.clone(), Arc::clone(&self.seams.renamer));
        let started = Edge::work("files-rename", move || {
            let heard_back = match renamer.rename(&folder, &from, &to) {
                // The watcher usually says so first; this makes sure the listing catches up.
                Ok(()) => Heard::Input(Input::Changed(Location::Folder(folder), Change::Rescan)),
                Err(err) => Heard::Told(err.to_string()),
            };
            let _ = heard.send(heard_back);
        });
        if started.is_err() {
            self.say(Heard::Told(RenameError::Other.to_string()));
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

/// When the window started, on the clock the window runs on (the wall clock, or a test's).
pub fn start() -> Instant {
    clock::now()
}

/// The browser's time: how long since `start`.
pub fn now(start: Instant) -> Now {
    Now(clock::since(start))
}
