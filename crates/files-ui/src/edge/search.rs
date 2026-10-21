//! The tag places: a walk of the home folder for the files that carry one colour, with its finds
//! streamed to the tab that asked as they come.

use std::cell::RefCell;
use std::collections::HashMap;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use files_core::{FolderPath, Location, TagColour};
use files_fs::{Flow, Found, WALK_CAP, Walked, find_tagged};
use files_model::{Input, TabId};

use super::{Edge, Heard};

/// The walks under way, by the tab and place they fill; each has a flag that stops it.
#[derive(Debug, Default)]
pub(super) struct Searches {
    stops: RefCell<HashMap<(TabId, Location), Arc<AtomicBool>>>,
}

impl Searches {
    /// Notes a new walk for `tab` at `place`, stopping the one it replaces. Returns its flag.
    fn begin(&self, tab: TabId, place: Location) -> Arc<AtomicBool> {
        let stop = Arc::new(AtomicBool::new(false));
        let old = self
            .stops
            .borrow_mut()
            .insert((tab, place), Arc::clone(&stop));
        if let Some(old) = old {
            old.store(true, Ordering::Relaxed);
        }
        stop
    }

    /// Stops the walk of `tab` at `place`, if there is one: the tab has left it.
    pub(super) fn stop(&self, tab: TabId, place: &Location) {
        let old = self.stops.borrow_mut().remove(&(tab, place.clone()));
        if let Some(old) = old {
            old.store(true, Ordering::Relaxed);
        }
    }
}

impl Edge {
    /// Walks `within` for the files tagged `colour`, off the window's thread. Each folder's finds
    /// are sent as `Input::Found`; the end is `Input::SearchDone` (not sent when the walk was
    /// stopped, because its tab left or asked again).
    pub(super) fn find(&self, tab: TabId, colour: TagColour, within: FolderPath) {
        let place = Location::Tag(colour);
        let flag = self.searches.begin(tab, place.clone());
        let (heard, lister) = (self.heard.clone(), Arc::clone(&self.lister));
        let walking = place.clone();
        let started = Edge::work("files-find", move || {
            let ended = find_tagged(&*lister, &within, colour, WALK_CAP, &mut |batch: Vec<
                Found,
            >| {
                if flag.load(Ordering::Relaxed) {
                    return Flow::Stop;
                }
                let rows = batch
                    .into_iter()
                    .map(|found| found.entry.found_at(found.item))
                    .collect();
                match heard.send(Heard::Tab(tab, Input::Found(walking.clone(), rows))) {
                    Ok(()) => Flow::Go,
                    // A closed channel means the window is gone.
                    Err(_) => Flow::Stop,
                }
            });
            if ended != Walked::Stopped && !flag.load(Ordering::Relaxed) {
                let _ = heard.send(Heard::Tab(tab, Input::SearchDone(walking)));
            }
        });
        if started.is_err() {
            self.say(Heard::Tab(tab, Input::SearchDone(place)));
        }
    }
}
