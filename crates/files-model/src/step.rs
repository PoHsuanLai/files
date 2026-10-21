//! The one step function: an input in, a new state and effects out.

use files_core::{Entry, FolderPrefs, HiddenFiles, Sort, ViewMode};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::input::{Change, Input, Now};
use crate::listing::{ListFailure, Listing};
use crate::selection::Selection;
use crate::typeselect::TypeSelect;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// The browser after `input` at time `now`, and what the caller should do about it.
    ///
    /// Answers for a place the browser has since left are ignored. While a name is being edited,
    /// the keys that would move the selection do nothing.
    pub fn step(self, input: Input, now: Now) -> (Browser, Vec<Effect>) {
        match input {
            Input::Click(..)
            | Input::SelectAll
            | Input::MoveCursor(..)
            | Input::Type(_)
            | Input::Open
                if self.rename.is_editing() =>
            {
                (self, Vec::new())
            }
            Input::Listed(at, result) if at == self.location => self.listed(result),
            Input::Changed(at, change) if at == self.location => self.changed(change),
            Input::PrefsLoaded(at, prefs) if at == self.location => self.with_prefs(prefs),
            Input::Listed(..) | Input::Changed(..) | Input::PrefsLoaded(..) => (self, Vec::new()),
            Input::Reveal { folder, names } => self.revealing(folder, names),
            Input::Go(to) => self.go(to),
            Input::Back => self.go_back(),
            Input::Forward => self.go_forward(),
            Input::Enclosing => self.go_enclosing(),
            Input::Reload => self.reload(),
            Input::Open => self.open(),
            Input::Click(name, modifier) => self.select(|s, rows| s.click(rows, &name, modifier)),
            Input::SelectAll => self.select(|s, rows| s.select_all(rows)),
            Input::SelectNone => self.select(|_, _| Selection::default()),
            Input::MoveCursor(direction, modifier) => {
                self.select(|s, rows| s.moved(rows, direction, modifier))
            }
            Input::Type(c) => self.type_char(c, now),
            Input::BeginRename => self.begin_rename(),
            Input::EditDraft(text) => self.edit_draft(text),
            Input::CommitRename => self.commit_rename(),
            Input::CancelRename => self.cancel_rename(),
            Input::SetView(view) => self.set_view(view),
            Input::SetSort(sort) => self.set_sort(sort),
            Input::ToggleHidden => self.toggle_hidden(),
        }
    }

    fn listed(self, result: Result<Vec<Entry>, ListFailure>) -> Stepped {
        let listing = match result {
            Ok(entries) => Listing::Loaded(entries),
            Err(failure) => Listing::Failed(failure),
        };
        (Browser { listing, ..self }.refreshed(), Vec::new())
    }

    fn changed(self, change: Change) -> Stepped {
        if change == Change::Rescan {
            return self.reload();
        }
        let Listing::Loaded(entries) = &self.listing else {
            return (self, Vec::new());
        };
        let listing = Listing::Loaded(crate::listing::apply(entries.clone(), change));
        (Browser { listing, ..self }.refreshed(), Vec::new())
    }

    fn with_prefs(self, prefs: FolderPrefs) -> Stepped {
        (Browser { prefs, ..self }.refreshed(), Vec::new())
    }

    fn set_view(self, view: ViewMode) -> Stepped {
        let mut prefs = self.prefs;
        prefs.view = view;
        self.saved(prefs)
    }

    fn set_sort(self, sort: Sort) -> Stepped {
        let mut prefs = self.prefs;
        prefs.sort = sort;
        self.saved(prefs)
    }

    fn saved(self, prefs: FolderPrefs) -> Stepped {
        let effects = vec![Effect::SavePrefs(self.location.clone(), prefs)];
        (Browser { prefs, ..self }.refreshed(), effects)
    }

    fn toggle_hidden(self) -> Stepped {
        let hidden = match self.hidden {
            HiddenFiles::Left => HiddenFiles::Shown,
            HiddenFiles::Shown => HiddenFiles::Left,
        };
        (Browser { hidden, ..self }.refreshed(), Vec::new())
    }

    /// Change the selection with `how`, and start a new word for type-select.
    fn select(self, how: impl FnOnce(Selection, &[Entry]) -> Selection) -> Stepped {
        let selection = how(self.selection, &self.rows);
        let browser = Browser {
            selection,
            typeselect: TypeSelect::default(),
            reveal: Vec::new(),
            ..self
        };
        (browser, Vec::new())
    }

    fn type_char(self, c: char, now: Now) -> Stepped {
        if c.is_control() {
            return (self, Vec::new());
        }
        let typeselect = self.typeselect.pushed(c, now);
        let found = typeselect
            .find(&self.rows)
            .and_then(|at| self.rows.get(at))
            .map(|row| Selection::only(&row.name));
        let selection = found.unwrap_or(self.selection);
        (
            Browser {
                typeselect,
                selection,
                reveal: Vec::new(),
                ..self
            },
            Vec::new(),
        )
    }
}
