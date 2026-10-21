//! Remembering how each folder is shown, on a thread of its own: reading or writing the file
//! never waits on the window, and a save and the load after it happen in the order they were asked.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, Sender, channel};

use files_core::{FolderPrefs, Location};
use files_fs::{PrefsError, PrefsStore};
use files_model::Input;
use tokio::sync::mpsc::UnboundedSender;

use super::Heard;

/// One thing asked of the store.
enum Job {
    /// How was this place shown?
    Load(Location),
    /// Remember how this place is shown.
    Save(Location, FolderPrefs),
}

/// The thread that serves a [`PrefsStore`].
pub(super) struct Prefs {
    /// `None` when the system would not start the thread: every place then opens with the
    /// defaults and nothing is remembered.
    jobs: Option<Sender<Job>>,
}

impl Prefs {
    /// Starts the thread over `store`; its answers go to `heard`.
    pub(super) fn start(store: Arc<dyn PrefsStore>, heard: UnboundedSender<Heard>) -> Prefs {
        let (jobs, inbox) = channel();
        let started = std::thread::Builder::new()
            .name("files-prefs".to_owned())
            .spawn(move || serve(&*store, &inbox, &heard));
        Prefs {
            jobs: started.ok().map(|_| jobs),
        }
    }

    /// Asks how `at` was last shown; the answer comes back as `Input::PrefsLoaded`.
    pub(super) fn load(&self, at: Location) {
        self.send(Job::Load(at));
    }

    /// Asks to remember that `at` is shown with `prefs`.
    pub(super) fn save(&self, at: Location, prefs: FolderPrefs) {
        self.send(Job::Save(at, prefs));
    }

    fn send(&self, job: Job) {
        if let Some(jobs) = &self.jobs {
            // A closed channel means the thread is gone.
            let _ = jobs.send(job);
        }
    }
}

/// Serves jobs until the edge is dropped.
fn serve(store: &dyn PrefsStore, inbox: &Receiver<Job>, heard: &UnboundedSender<Heard>) {
    while let Ok(job) = inbox.recv() {
        match job {
            Job::Load(at) => {
                let prefs = match &at {
                    Location::Folder(folder) => store.load(folder),
                    Location::Trash | _ => FolderPrefs::default(),
                };
                let _ = heard.send(Heard::Input(Input::PrefsLoaded(at, prefs)));
            }
            Job::Save(Location::Folder(folder), prefs) => match store.save(&folder, prefs) {
                // The file is there but unusable: the view works, it just is not remembered.
                Ok(()) | Err(PrefsError::Unusable) => {}
                Err(failure) => {
                    let _ = heard.send(Heard::Told(failure.to_string()));
                }
            },
            // The Trash is always shown the same way.
            Job::Save(Location::Trash | _, _) => {}
        }
    }
}
