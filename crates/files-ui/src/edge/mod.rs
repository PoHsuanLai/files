//! The window's edges: what carries out the model's [`Effect`]s against the disk and the desktop,
//! and what the window reads from outside (the clock).
//!
//! [`Edge::run`] takes the effects a step returned. Listing, watching and renaming run on threads
//! of their own and answer on a channel as [`Heard`]; the window's loop (`state`) feeds them back
//! to the browser. Everything ambient is read here and in [`Zone`], once, and handed down as values.
//!
//! Every effect arrives tagged with the tab that asked for it ([`Tabbed`]), and every answer goes
//! back tagged with that tab ([`Heard::Tab`]), so a listing asked for by a tab in the background
//! lands in that tab; what is no tab's (the disks changing, a folder rescanned after a change) is
//! [`Heard::All`]. Reading the disk (`read`), the queue (`ops`), tags (`tagging`), searches
//! (`search`), the disks (`volumes`) and the seams themselves (`seams`) each have a module; this one
//! dispatches.
//!
//! Changes to the files (`Effect::Run`, `Undo`, `Answer`, `Cancel`) go to a [`files_ops::Worker`],
//! one queue on a thread of its own (`ops`). Each job is tagged with the tab that asked for it, and
//! all its events (started, progress, a question, finished) come back to that tab as `Input::Op`;
//! each finished change is followed by a rescan of the places it touched, told to every tab. The
//! Trash is listed from [`files_trash::Trash`].
//!
//! A folder's view and sort are remembered through [`files_fs::PrefsStore`] on a thread of their own
//! (`prefs`); Quick Look's previews come from anyview's peek worker (`look`) and arrive as
//! [`Heard::Looked`], not as an input: a picture is not something the model holds. Get Info's facts
//! are read by the [`Lister`] on a thread and come back as `Input::InfoRead`.

mod apps;
#[cfg(feature = "testing")]
mod fake_volumes;
mod look;
mod opener;
mod ops;
mod prefs;
mod read;
mod seams;
mod search;
mod tagging;
mod volumes;
mod zone;

use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;
use std::time::Instant;

use crate::words::volume_trouble_words;
use anyview_peek::{AnyPeeked, PeekWorker};
use ds::base::time::clock;
use ds_blitz::clipboard;
use files_core::{ItemRef, Location, Summary};
use files_fs::{Lister, Tagged, WatchHandle};
use files_model::{Effect, Input, Now, OpenTarget, TabId, Tabbed};
use files_ops::{Job, Worker};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

pub use apps::{AppChoiceView, AppRole, Apps, NoApps};
#[cfg(feature = "testing")]
pub use fake_volumes::{FakeVolumes, VolumeCall};
pub use opener::{OpenAs, Opener};
pub use read::change_of;
pub use seams::Seams;
pub use volumes::{Changed, NoVolumes, Volumes, WatchGuard};
pub use zone::Zone;

/// What the person reads when the clipboard would not take the pathnames.
const COPY_FAILED: &str = "The pathnames couldn\u{2019}t be copied.";

/// What a worker tells the window.
#[derive(Debug, Clone, PartialEq)]
pub enum Heard {
    /// An answer for the tab that asked.
    Tab(TabId, Input),
    /// News for every tab (the disks changed, a place was rescanned).
    All(Input),
    /// The last tab was closed: close the window.
    CloseWindow,
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
    /// The lister of the seams, with every entry carrying its tags.
    lister: Arc<dyn Lister>,
    heard: UnboundedSender<Heard>,
    watches: RefCell<HashMap<(TabId, Location), WatchHandle>>,
    /// The searches under way, to stop the one a tab leaves.
    searches: search::Searches,
    /// The queue of changes, each job tagged with the tab that asked for it; `None` when the
    /// system would not start its thread.
    ops: Option<Worker<TabId>>,
    /// Remembers how each folder is shown.
    prefs: prefs::Prefs,
    /// Makes Quick Look's previews.
    peek: PeekWorker,
    /// Lists, opens and ejects the disks.
    disks: volumes::Disks,
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
        let sink = ops::sink(heard.clone());
        let ops = Worker::start(Arc::clone(&seams.trash), sink).ok();
        let lister: Arc<dyn Lister> = Arc::new(Tagged::new(
            Arc::clone(&seams.lister),
            Arc::clone(&seams.tagger),
        ));
        let prefs = prefs::Prefs::start(Arc::clone(&seams.prefs), heard.clone());
        let disks = volumes::Disks::start(Arc::clone(&seams.volumes), heard.clone());
        let edge = Edge {
            seams,
            lister,
            heard,
            watches: RefCell::new(HashMap::new()),
            searches: search::Searches::default(),
            ops,
            prefs,
            peek: look::worker(),
            disks,
        };
        (edge, inbox)
    }

    /// Carries out `effects`, in order. Work that takes time runs on a thread.
    pub fn run(&self, effects: Vec<Tabbed>) {
        for Tabbed { tab, effect, .. } in effects {
            self.carry_out(tab, effect);
        }
    }

    /// Carries out `effects` that `tab` asked for.
    pub fn run_in(&self, tab: TabId, effects: Vec<Effect>) {
        self.run(Tabbed::all(tab, effects));
    }

    fn carry_out(&self, tab: TabId, effect: Effect) {
        match effect {
            Effect::List(at) => self.list(tab, at),
            Effect::Find { colour, within } => self.find(tab, colour, within),
            Effect::Watch(at) => self.watch(tab, at),
            Effect::Unwatch(at) => {
                self.searches.stop(tab, &at);
                self.watches.borrow_mut().remove(&(tab, at));
            }
            Effect::OpenFiles(targets) => {
                self.open(&targets, OpenAs::Default);
            }
            Effect::OpenFilesWith { targets, app } => self.open(&targets, OpenAs::App(app)),
            Effect::CheckFolder(folder) => self.check(tab, folder),
            Effect::CopyText(text) => {
                if clipboard::write_text(&text).is_err() {
                    self.say(Heard::Told(COPY_FAILED.to_owned()));
                }
            }
            Effect::Rename { folder, from, to } => self.rename(tab, folder, from, to),
            Effect::Run(plan) => self.submit(tab, Job::Run(plan)),
            Effect::Undo => self.submit(tab, Job::Undo),
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
            Effect::LoadPrefs(at) => self.prefs.load(tab, at),
            Effect::SavePrefs(at, prefs) => self.prefs.save(at, prefs),
            Effect::Look(item) => look::ask(&self.peek, item, self.heard.clone()),
            Effect::ReadInfo(item) => self.read_info(tab, item),
            Effect::Tag { items, colour, how } => self.tag(items, colour, how),
            Effect::CloseWindow => self.say(Heard::CloseWindow),
            Effect::MountVolume(id) => self.disks.mount(tab, id),
            Effect::EjectVolume(id) => self.disks.eject(tab, id),
            Effect::ForceEjectVolume(id) => self.disks.force_eject(tab, id),
            Effect::TellVolume { name, trouble } => {
                self.say(Heard::Told(volume_trouble_words(&name, &trouble)));
            }
            // An effect files-model adds later is dropped until this names it.
            _ => {}
        }
    }

    /// Hands `targets` to the opener, and tells the person if it could not.
    fn open(&self, targets: &[OpenTarget], with: OpenAs) {
        let paths = targets.iter().map(OpenTarget::path).collect();
        if let Err(failure) = self.seams.opener.open(paths, with) {
            self.say(Heard::Told(failure.to_string()));
        }
    }

    /// The apps that open every one of `targets`, for Open With.
    pub fn apps_for(&self, targets: &[OpenTarget]) -> Vec<AppChoiceView> {
        let paths: Vec<_> = targets.iter().map(OpenTarget::path).collect();
        self.seams.apps.apps_for(&paths)
    }

    /// Every installed app.
    pub fn all_apps(&self) -> Vec<AppChoiceView> {
        self.seams.apps.all_apps()
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
}

/// When the window started, on the clock the window runs on (the wall clock, or a test's).
pub fn start() -> Instant {
    clock::now()
}

/// The browser's time: how long since `start`.
pub fn now(start: Instant) -> Now {
    Now(clock::since(start))
}
