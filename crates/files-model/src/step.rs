//! The one step function: an input in, a new state and effects out.

use files_core::{Entry, FolderPrefs, HiddenFiles, Sort, ViewMode};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::error::ListFailure;
use crate::input::{Change, Input, Now};
use crate::listing::Listing;
use crate::selection::Selection;
use crate::typeselect::TypeSelect;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// The browser after `input` at time `now`, and what the caller should do about it.
    ///
    /// Answers for a place the browser has since left are ignored. While a name is being edited,
    /// the keys that would move the selection do nothing.
    pub fn step(self, input: Input, now: Now) -> (Browser, Vec<Effect>) {
        let showing = self.quick_item();
        let (next, mut effects) = self.stepped(input, now);
        // Quick Look follows the cursor: whenever it lands on a new item, ask for its preview.
        if let Some(item) = next
            .quick_item()
            .filter(|item| Some(item) != showing.as_ref())
        {
            effects.push(Effect::Look(item));
        }
        (next, effects)
    }

    fn stepped(self, input: Input, now: Now) -> Stepped {
        match input {
            Input::Click(..)
            | Input::SelectAll
            | Input::MoveCursor(..)
            | Input::Type(_)
            | Input::Open
            | Input::OpenWith(_)
                if self.rename.is_editing() =>
            {
                (self, Vec::new())
            }
            Input::Listed(at, result) if at == self.location => self.listed(result),
            Input::Changed(at, change) if at == self.location => self.changed(change),
            Input::PrefsLoaded(at, prefs) if at == self.location => self.with_prefs(prefs),
            Input::Found(at, batch) if at == self.location => self.found(batch),
            Input::SearchDone(at) if at == self.location => self.search_done(),
            Input::Listed(..)
            | Input::Changed(..)
            | Input::PrefsLoaded(..)
            | Input::Found(..)
            | Input::SearchDone(..) => (self, Vec::new()),
            Input::Reveal { folder, names } => self.revealing(folder, names),
            Input::Go(to) => self.go(to),
            Input::Properties { folder, names } => self.properties(folder, names),
            Input::QuickLook => self.toggle_quick_look(),
            Input::CloseQuickLook => self.close_quick_look(),
            Input::GetInfo => self.get_info(),
            Input::InfoRead(item, result) => self.info_read(item, result),
            Input::CloseInfo => self.close_info(),
            Input::Back => self.go_back(),
            Input::Forward => self.go_forward(),
            Input::Enclosing => self.go_enclosing(),
            Input::Reload => self.reload(),
            Input::Open => self.open(),
            Input::OpenWith(app) => self.open_with(app),
            Input::ChooseApp => self.choose_app(),
            Input::CloseChooseApp => self.close_chooser(),
            Input::GoToFolder => self.go_to_folder(),
            Input::GoToTyped(text) => self.go_to_typed(text),
            Input::GoToSubmit => self.go_to_submit(),
            Input::GoToCancel => self.go_to_cancel(),
            Input::FolderChecked(folder, result) => self.folder_checked(folder, result),
            Input::GetInfoFolder => self.get_info_folder(),
            Input::CopyAsPathname => self.copy_as_pathname(),
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
            Input::NewFolder => self.new_folder(),
            Input::MoveToTrash => self.move_to_trash(),
            Input::PutBack => self.put_back(),
            Input::DeleteImmediately => self.delete_immediately(),
            Input::EmptyTrash => self.empty_trash(),
            Input::Confirm => self.confirm(),
            Input::Dismiss => self.dismiss(),
            Input::Answer(resolution) => self.answer(resolution),
            Input::Copy => self.copy(),
            Input::Paste => self.paste(),
            Input::PasteMove => self.paste_move(),
            Input::Drop { sources, onto, how } => self.dropped(sources, onto, how),
            Input::Undo => self.undo(),
            Input::CancelWork => self.cancel_work(),
            Input::Op(event) => self.heard(event),
            Input::Volumes(volumes) => self.volumes_changed(volumes),
            Input::OpenVolume(id) => self.open_volume(id),
            Input::Mounted(id, result) => self.mounted(id, result),
            Input::Eject(id) => self.eject(id),
            Input::Ejected(id, result) => self.ejected(id, result),
            Input::ForceEjected(id, result) => self.force_ejected(id, result),
            Input::TrashItems(items) => self.trash_items(items),
            Input::MakeFolder { parent, name } => self.make_folder(parent, name),
            Input::Bring {
                sources,
                destination,
                how,
            } => self.bring(sources, destination, how),
            Input::AskEmptyTrash => self.ask_empty_trash(),
            Input::Tag(colour) => self.tag(colour),
            // The tabs are `Tabs`'; a lone browser has none.
            Input::NewTab
            | Input::OpenInNewTab
            | Input::CloseTab
            | Input::NextTab
            | Input::PreviousTab
            | Input::ShowTab(_) => (self, Vec::new()),
        }
    }

    fn found(self, batch: Vec<Entry>) -> Stepped {
        let listing = match self.listing {
            Listing::Loading => Listing::Searching(batch),
            Listing::Searching(mut so_far) => {
                so_far.extend(batch);
                Listing::Searching(so_far)
            }
            kept @ (Listing::Loaded(_) | Listing::Failed(_)) => kept,
        };
        (Browser { listing, ..self }.refreshed(), Vec::new())
    }

    fn search_done(self) -> Stepped {
        let listing = match self.listing {
            Listing::Loading => Listing::Loaded(Vec::new()),
            Listing::Searching(found) => Listing::Loaded(found),
            kept @ (Listing::Loaded(_) | Listing::Failed(_)) => kept,
        };
        (Browser { listing, ..self }.refreshed(), Vec::new())
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
