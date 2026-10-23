//! One window's view of one location.

use files_core::{
    Behaviour, Entry, FileName, FolderPath, FolderPrefs, HiddenFiles, Info, ItemRef, Location,
    Summary, Volume,
};

use crate::app::Chooser;
use crate::arrange::arrange;
use crate::effect::Effect;
use crate::goto::GoTo;
use crate::history::{Availability, History};
use crate::listing::Listing;
use crate::look::{InfoView, Looking};
use crate::ops::{Question, Work};
use crate::preview::Pane;
use crate::rename::Rename;
use crate::reveal;
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
    /// The person's choices about how a window behaves (the window's, shared by its tabs).
    pub(crate) behaviour: Behaviour,
    /// The names an `Input::Reveal` asked to select, until the person selects something.
    pub(crate) reveal: Vec<FileName>,
    /// What is being changed now.
    pub(crate) work: Work,
    /// What would be undone now, if anything.
    pub(crate) undo: Option<Summary>,
    /// What the window is waiting for an answer to.
    pub(crate) question: Question,
    /// What was copied, for a paste.
    pub(crate) clipboard: Vec<ItemRef>,
    /// The folder just made, whose name is opened for editing once it is listed.
    pub(crate) starting: Option<FileName>,
    /// Whether Quick Look is up.
    pub(crate) looking: Looking,
    /// The Get Info panel.
    pub(crate) info: InfoView,
    /// Whether the preview pane is up (the window's, shared by its tabs).
    pub(crate) pane: Pane,
    /// The facts read for the item the pane shows, once they have arrived.
    pub(crate) details: Option<Info>,
    /// The person's home folder, which `~` means in Go to Folder; `None` until the caller says.
    pub(crate) home: Option<FolderPath>,
    /// The Go to Folder sheet.
    pub(crate) goto: GoTo,
    /// The sheet of every installed app.
    pub(crate) chooser: Chooser,
    /// The disks the sidebar lists, as the caller last told.
    pub(crate) volumes: Vec<Volume>,
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
            behaviour: Behaviour::default(),
            reveal: Vec::new(),
            work: Work::Idle,
            undo: None,
            question: Question::None,
            clipboard: Vec::new(),
            starting: None,
            looking: Looking::Closed,
            info: InfoView::Closed,
            pane: Pane::Hidden,
            details: None,
            home: None,
            goto: GoTo::Closed,
            chooser: Chooser::Closed,
            volumes: Vec::new(),
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

    /// How the window behaves where the person has a choice.
    pub fn behaviour(&self) -> Behaviour {
        self.behaviour
    }

    /// The letters typed to jump to a name, lower-cased; empty before the first.
    pub fn typed(&self) -> &str {
        self.typeselect.prefix()
    }

    /// The change being made now, if any.
    pub fn work(&self) -> &Work {
        &self.work
    }

    /// What the window is waiting for an answer to.
    pub fn question(&self) -> &Question {
        &self.question
    }

    /// What undoing would take back now, if anything.
    pub fn undo_summary(&self) -> Option<&Summary> {
        self.undo.as_ref()
    }

    /// Whether Undo works.
    pub fn can_undo(&self) -> Availability {
        match self.undo {
            Some(_) => Availability::Available,
            None => Availability::Unavailable,
        }
    }

    /// Whether Paste works: something was copied and this is a folder.
    pub fn can_paste(&self) -> Availability {
        match (self.folder(), self.clipboard.is_empty()) {
            (Some(_), false) => Availability::Available,
            _ => Availability::Unavailable,
        }
    }

    /// What was copied, for a paste.
    pub fn clipboard(&self) -> &[ItemRef] {
        &self.clipboard
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

    /// What to ask for on arriving at `to`: its listing, its changes and how it was shown.
    pub(crate) fn entering(to: &Location) -> Vec<Effect> {
        vec![
            Effect::List(to.clone()),
            Effect::Watch(to.clone()),
            Effect::LoadPrefs(to.clone()),
        ]
    }

    /// A new browser at `location` for a new tab: it knows what this one knows of the person's
    /// home, the disks, hidden files, how the window behaves, what was copied and what can be undone, and starts with
    /// nothing else.
    pub(crate) fn opened_like(&self, location: Location) -> (Browser, Vec<Effect>) {
        let (browser, _) = Browser::new(location.clone());
        let browser = Browser {
            home: self.home.clone(),
            hidden: self.hidden,
            behaviour: self.behaviour,
            volumes: self.volumes.clone(),
            pane: self.pane,
            clipboard: self.clipboard.clone(),
            undo: self.undo.clone(),
            ..browser
        };
        let effects = Browser::entering(&location);
        (browser, effects)
    }

    /// The item a row stands for, in the folder shown. `None` for a row of the Trash.
    pub(crate) fn item_of(&self, row: &Entry) -> Option<ItemRef> {
        let folder = self.folder()?;
        Some(ItemRef::new(folder.clone(), row.name.clone()))
    }

    /// The selected rows with the items they stand for, in shown order.
    pub(crate) fn selected_rows(&self) -> Vec<(&Entry, ItemRef)> {
        self.rows
            .iter()
            .filter(|row| self.selection.contains(&row.name))
            .filter_map(|row| self.item_of(row).map(|item| (row, item)))
            .collect()
    }

    /// The browser with its rows, selection and rename brought in line with its listing, prefs and
    /// hidden-files choice.
    pub(crate) fn refreshed(self) -> Browser {
        let entries = self.listing.entries();
        let order = arrange(entries, &self.prefs, self.hidden, self.behaviour.folders);
        let rows: Vec<Entry> = order
            .into_iter()
            .filter_map(|at| entries.get(at).cloned())
            .collect();
        let rename = self.rename.kept_in(entries);
        let selection = self.selection.kept_in(&rows);
        let selection = reveal::applied(&self.listing, &rows, &self.reveal, selection);
        let (starting, selection, rename) =
            Browser::started(self.starting.clone(), &rows, selection, rename);
        // Quick Look has nothing to show once the row it was over is gone.
        let looking = match (self.looking, selection.cursor()) {
            (Looking::Open, Some(_)) => Looking::Open,
            (Looking::Open | Looking::Closed, _) => Looking::Closed,
        };
        Browser {
            rows,
            selection,
            rename,
            starting,
            looking,
            ..self
        }
    }
}
