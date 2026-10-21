//! The window's edges: what carries out the model's [`Effect`]s against the disk and the desktop,
//! and what the window reads from outside (the clock).
//!
//! [`Edge::run`] takes the effects a step returned. Listing, watching and renaming run on threads
//! of their own and answer on a channel as [`Heard`]; the window's loop (`state`) feeds them back
//! to the browser. Everything ambient is read here and in [`Zone`], once, and handed down as values.
//!
//! Changes to the files (`Effect::Run`, `Undo`, `Answer`, `Cancel`) go to a [`files_ops::Worker`],
//! one queue on a thread of its own; its events come back as `Input::Op`, and each finished
//! change is followed by a rescan of the places it touched. The Trash is listed from
//! [`files_trash::Trash`].
//!
//! A folder's view and sort are remembered through [`PrefsStore`] on a thread of their own
//! (`prefs`); Quick Look's previews come from anyview's peek worker (`look`) and arrive as
//! [`Heard::Looked`], not as an input: a picture is not something the model holds. Get Info's facts
//! are read by the [`Lister`] on a thread and come back as `Input::InfoRead`.

mod look;
mod opener;
mod ops;
mod prefs;
mod zone;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use anyview_peek::{AnyPeeked, PeekWorker};
use ds::base::time::clock;
use files_core::{FileName, FolderPath, ItemRef, Location, OpFailure, OpPlan, Summary};
use files_fs::{
    DiskLister, FolderEvent, ListError, Lister, NoPrefs, NotifyWatcher, PrefsStore, WatchHandle,
    Watcher,
};
use files_model::{Change, Effect, Input, ListFailure, Now, OpenTarget};
use files_ops::{Job, Worker};
use files_trash::{SystemTrash, Trash};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub use opener::Opener;
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
    /// The Trash: moving to it, listing it, putting back, erasing.
    pub trash: Arc<dyn Trash>,
    /// Where each folder's view and sort are remembered.
    pub prefs: Arc<dyn PrefsStore>,
}

impl Seams {
    /// The real disk and Trash, with `opener` to open files.
    pub fn disk(opener: Arc<dyn Opener>) -> Seams {
        Seams {
            lister: Arc::new(DiskLister),
            watcher: Arc::new(NotifyWatcher),
            opener,
            trash: Arc::new(SystemTrash),
            prefs: Arc::new(NoPrefs),
        }
    }

    /// The same seams remembering folder views in `prefs`.
    pub fn with_prefs(self, prefs: Arc<dyn PrefsStore>) -> Seams {
        Seams { prefs, ..self }
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
    /// A change was made that can be undone (a move to the Trash): said with an Undo button.
    Announced(Summary),
    /// The preview of an item, for Quick Look.
    Looked(ItemRef, Arc<AnyPeeked>),
}

/// Carries out effects. Lives on the window's thread; its workers answer on the channel it made.
pub struct Edge {
    seams: Seams,
    heard: UnboundedSender<Heard>,
    watches: RefCell<HashMap<Location, WatchHandle>>,
    /// The queue of changes; `None` when the system would not start its thread.
    ops: Option<Worker>,
    /// Remembers how each folder is shown.
    prefs: prefs::Prefs,
    /// Makes Quick Look's previews.
    peek: PeekWorker,
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
        let ops = Worker::start(Arc::clone(&seams.trash), ops::sink(heard.clone())).ok();
        let prefs = prefs::Prefs::start(Arc::clone(&seams.prefs), heard.clone());
        let edge = Edge {
            seams,
            heard,
            watches: RefCell::new(HashMap::new()),
            ops,
            prefs,
            peek: look::worker(),
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
                if let Err(failure) = self.seams.opener.open(paths) {
                    self.say(Heard::Told(failure.to_string()));
                }
            }
            Effect::Rename { folder, from, to } => self.rename(folder, from, to),
            Effect::Run(plan) => self.submit(Job::Run(plan)),
            Effect::Undo => self.submit(Job::Undo),
            Effect::Answer(resolution) => {
                if let Some(queue) = &self.ops {
                    queue.answer(resolution);
                }
            }
            Effect::Cancel => {
                if let Some(queue) = &self.ops {
                    queue.cancel();
                }
            }
            Effect::Tell(failure) => self.say(Heard::Told(failure.to_string())),
            Effect::Announce(summary) => self.say(Heard::Announced(summary)),
            Effect::LoadPrefs(at) => self.prefs.load(at),
            Effect::SavePrefs(at, prefs) => self.prefs.save(at, prefs),
            Effect::Look(item) => look::ask(&self.peek, item, self.heard.clone()),
            Effect::ReadInfo(item) => self.read_info(item),
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
        let trash = Arc::clone(&self.seams.trash);
        let failed = at.clone();
        let started = Edge::work("files-list", move || {
            let result = match &at {
                Location::Folder(folder) => lister.list(folder).map_err(failure_of),
                Location::Trash => trash
                    .list()
                    .map(|items| items.into_iter().filter_map(ops::row_of).collect())
                    .map_err(ops::failure_of),
                _ => Err(ListFailure::Other),
            };
            let _ = heard.send(Heard::Input(Input::Listed(at, result)));
        });
        if started.is_err() {
            self.say(Heard::Input(Input::Listed(failed, Err(ListFailure::Other))));
        }
    }

    /// Reads the facts of `item` for Get Info, off the window's thread.
    fn read_info(&self, item: ItemRef) {
        let (heard, lister) = (self.heard.clone(), Arc::clone(&self.seams.lister));
        let failed = item.clone();
        let started = Edge::work("files-info", move || {
            let result = lister.info(&item).map_err(failure_of);
            let _ = heard.send(Heard::Input(Input::InfoRead(item, result)));
        });
        if started.is_err() {
            self.say(Heard::Input(Input::InfoRead(
                failed,
                Err(ListFailure::Other),
            )));
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

    /// A rename is a change like any other, so it can be undone.
    fn rename(&self, folder: FolderPath, from: FileName, to: FileName) {
        let item = ItemRef::new(folder, from);
        self.submit(Job::Run(OpPlan::Rename { item, to }));
    }

    /// Puts `job` on the queue of changes, or says the queue could not be started.
    fn submit(&self, job: Job) {
        match &self.ops {
            Some(queue) => queue.submit(job),
            None => self.say(Heard::Told(OpFailure::Other.to_string())),
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
