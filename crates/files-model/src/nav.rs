//! Going places and opening things.

use files_core::Location;

use crate::app::Chooser;
use crate::browser::Browser;
use crate::effect::{Effect, OpenTarget};
use crate::goto::GoTo;
use crate::history::History;
use crate::listing::Listing;
use crate::look::{InfoView, Looking};
use crate::open::{Opens, opens};
use crate::rename::Rename;
use crate::selection::Selection;
use crate::typeselect::TypeSelect;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// Leave this place for `to`, with `history` already updated. Everything about the old place
    /// is forgotten, except whether hidden files are shown.
    fn arrive(self, to: Location, history: History) -> Stepped {
        let mut effects = vec![Effect::Unwatch(self.location.clone())];
        effects.extend(Browser::entering(&to, self.home.as_ref()));
        let browser = Browser {
            location: to,
            history,
            listing: Listing::Loading,
            rows: Vec::new(),
            selection: Selection::default(),
            typeselect: TypeSelect::default(),
            rename: Rename::Idle,
            prefs: Default::default(),
            hidden: self.hidden,
            reveal: Vec::new(),
            work: self.work,
            undo: self.undo,
            question: self.question,
            clipboard: self.clipboard,
            starting: None,
            looking: Looking::Closed,
            info: InfoView::Closed,
            home: self.home,
            goto: GoTo::Closed,
            chooser: Chooser::Closed,
            volumes: self.volumes,
        };
        (browser, effects)
    }

    /// Read this place again.
    pub(crate) fn reload(self) -> Stepped {
        let effects = vec![Browser::reading(&self.location, self.home.as_ref())];
        match self.location {
            // A search starts over, so what it finds again is not listed twice.
            Location::Tag(_) => (
                Browser {
                    listing: Listing::Loading,
                    ..self
                }
                .refreshed(),
                effects,
            ),
            Location::Folder(_) | Location::Trash | _ => (self, effects),
        }
    }

    /// Go to a new place; going where it already is reads it again.
    pub(crate) fn go(self, to: Location) -> Stepped {
        if to == self.location {
            return self.reload();
        }
        let history = self.history.visited(self.location.clone());
        self.arrive(to, history)
    }

    /// Go back, if there is a place behind.
    pub(crate) fn go_back(self) -> Stepped {
        match self.history.went_back(&self.location) {
            Some((history, to)) => self.arrive(to, history),
            None => (self, Vec::new()),
        }
    }

    /// Go forward, if there is a place ahead.
    pub(crate) fn go_forward(self) -> Stepped {
        match self.history.went_forward(&self.location) {
            Some((history, to)) => self.arrive(to, history),
            None => (self, Vec::new()),
        }
    }

    /// Go to the folder above, if there is one.
    pub(crate) fn go_enclosing(self) -> Stepped {
        match self.enclosing() {
            Some(to) => self.go(to),
            None => (self, Vec::new()),
        }
    }

    /// The folder that opening the selection goes into: the selection is that one folder (or a
    /// link to one).
    pub(crate) fn lone_folder(&self) -> Option<Location> {
        match self.selected_rows().as_slice() {
            [(only, item)] if opens(only) == Opens::Folder => {
                Some(Location::Folder(item.as_folder()))
            }
            _ => None,
        }
    }

    /// Open the selection: a lone folder is entered, files go to their apps, and anything else
    /// does nothing.
    pub(crate) fn open(self) -> Stepped {
        let into = self.lone_folder();
        let files: Vec<OpenTarget> = self
            .selected_rows()
            .into_iter()
            .filter(|(row, _)| opens(row) == Opens::File)
            .map(|(_, item)| OpenTarget::new(item.folder, item.name))
            .collect();
        match (into, files.is_empty()) {
            (Some(to), _) => self.go(to),
            (None, true) => (self, Vec::new()),
            (None, false) => (self, vec![Effect::OpenFiles(files)]),
        }
    }
}
