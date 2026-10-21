//! One window's view of one location.

use files_core::{Entry, FolderPath, FolderPrefs, HiddenFiles, Location};

use crate::arrange::arrange;
use crate::effect::Effect;
use crate::history::{Availability, History};
use crate::listing::Listing;
use crate::rename::Rename;
use crate::selection::Selection;
use crate::typeselect::TypeSelect;

/// The state of one browser: a location, the way back to it, what it holds and what the person is
/// doing with it. Change it only with [`Browser::step`].
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Browser {
    pub(crate) location: Location,
    pub(crate) history: History,
    pub(crate) listing: Listing,
    pub(crate) rows: Vec<Entry>,
    pub(crate) selection: Selection,
    pub(crate) typeselect: TypeSelect,
    pub(crate) rename: Rename,
    pub(crate) prefs: FolderPrefs,
    pub(crate) hidden: HiddenFiles,
}

impl Browser {
    /// A browser at `location` with nothing loaded yet, and the effects that load it.
    pub fn new(location: Location) -> (Browser, Vec<Effect>) {
        let effects = Browser::entering(&location);
        let browser = Browser {
            location,
            history: History::default(),
            listing: Listing::Loading,
            rows: Vec::new(),
            selection: Selection::default(),
            typeselect: TypeSelect::default(),
            rename: Rename::Idle,
            prefs: FolderPrefs::default(),
            hidden: HiddenFiles::default(),
        };
        (browser, effects)
    }

    /// Where the browser is.
    pub fn location(&self) -> &Location {
        &self.location
    }

    /// The folder, when the location is one on disk.
    pub fn folder(&self) -> Option<&FolderPath> {
        if let Location::Folder(path) = &self.location {
            Some(path)
        } else {
            None
        }
    }

    /// What was last read of the location.
    pub fn listing(&self) -> &Listing {
        &self.listing
    }

    /// The rows to show, in order: hidden entries left out unless shown, sorted.
    pub fn rows(&self) -> &[Entry] {
        &self.rows
    }

    /// What is selected.
    pub fn selection(&self) -> &Selection {
        &self.selection
    }

    /// The row the keyboard is on.
    pub fn cursor(&self) -> Option<&files_core::FileName> {
        self.selection.cursor()
    }

    /// The rename in progress, if any.
    pub fn rename(&self) -> &Rename {
        &self.rename
    }

    /// How this folder is shown.
    pub fn prefs(&self) -> FolderPrefs {
        self.prefs
    }

    /// Whether hidden files are shown.
    pub fn hidden(&self) -> HiddenFiles {
        self.hidden
    }

    /// The letters typed to jump to a name, lower-cased; empty before the first.
    pub fn typed(&self) -> &str {
        self.typeselect.prefix()
    }

    /// The places behind and ahead.
    pub fn history(&self) -> &History {
        &self.history
    }

    /// Whether Back works.
    pub fn can_go_back(&self) -> Availability {
        self.history.back_availability()
    }

    /// Whether Forward works.
    pub fn can_go_forward(&self) -> Availability {
        self.history.forward_availability()
    }

    /// Whether Enclosing Folder works.
    pub fn can_go_enclosing(&self) -> Availability {
        match self.enclosing() {
            Some(_) => Availability::Available,
            None => Availability::Unavailable,
        }
    }

    /// The location above this one.
    pub(crate) fn enclosing(&self) -> Option<Location> {
        self.folder()?.parent().map(Location::Folder)
    }

    /// What to ask for on arriving at `to`.
    pub(crate) fn entering(to: &Location) -> Vec<Effect> {
        vec![
            Effect::List(to.clone()),
            Effect::Watch(to.clone()),
            Effect::LoadPrefs(to.clone()),
        ]
    }

    /// The browser with its rows, selection and rename brought in line with its listing, prefs and
    /// hidden-files choice.
    pub(crate) fn refreshed(self) -> Browser {
        let entries = self.listing.entries();
        let rows: Vec<Entry> = arrange(entries, &self.prefs, self.hidden)
            .into_iter()
            .filter_map(|at| entries.get(at).cloned())
            .collect();
        let rename = self.rename.kept_in(entries);
        let selection = self.selection.kept_in(&rows);
        Browser {
            rows,
            selection,
            rename,
            ..self
        }
    }
}
