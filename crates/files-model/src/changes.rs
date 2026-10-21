//! Changing files: new folder, Trash, copy and paste, drops, undo, and the workers' reports.

use files_core::{
    Answer, FileName, ItemRef, Location, OpKind, OpPlan, Resolution, Split, Transfer, TrashedItem,
    keep_both,
};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::open::{Opens, opens};
use crate::ops::{DropHow, End, OpEvent, Question, Work};
use crate::rename::Rename;
use crate::selection::Selection;

type Stepped = (Browser, Vec<Effect>);

/// The name a new folder starts with.
const NEW_FOLDER: &str = "untitled folder";

impl Browser {
    /// The selected items, in row order, when this is a folder.
    fn selected_items(&self) -> Vec<ItemRef> {
        let Some(folder) = self.folder() else {
            return Vec::new();
        };
        self.rows
            .iter()
            .filter(|row| self.selection.contains(&row.name))
            .map(|row| ItemRef::new(folder.clone(), row.name.clone()))
            .collect()
    }

    /// The selected items of the Trash, in row order.
    fn selected_trashed(&self) -> Vec<TrashedItem> {
        if self.location != Location::Trash {
            return Vec::new();
        }
        self.rows
            .iter()
            .filter(|row| self.selection.contains(&row.name))
            .filter_map(|row| row.trashed.clone())
            .collect()
    }

    fn run(self, plan: OpPlan) -> Stepped {
        (self, vec![Effect::Run(plan)])
    }

    /// Make a folder called `untitled folder` (or the next free `untitled folder 2`), and open its
    /// name for editing once it is listed.
    pub(crate) fn new_folder(self) -> Stepped {
        let Some(parent) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        let entries = self.listing.entries();
        let taken = |name: &FileName| entries.iter().any(|e| e.name == *name);
        let Ok(base) = FileName::new(NEW_FOLDER) else {
            return (self, Vec::new());
        };
        let Ok(name) = keep_both(&base, Split::Whole, taken) else {
            return (self, Vec::new());
        };
        let browser = Browser {
            starting: Some(name.clone()),
            ..self
        };
        browser.run(OpPlan::NewFolder { parent, name })
    }

    /// Move the selection to the Trash.
    pub(crate) fn move_to_trash(self) -> Stepped {
        let items = self.selected_items();
        if items.is_empty() {
            return (self, Vec::new());
        }
        self.run(OpPlan::Trash { items })
    }

    /// Put the selected Trash items back where they came from.
    pub(crate) fn put_back(self) -> Stepped {
        let items = self.selected_trashed();
        if items.is_empty() {
            return (self, Vec::new());
        }
        self.run(OpPlan::PutBack { items })
    }

    /// Ask whether to erase the selected Trash items for good.
    pub(crate) fn delete_immediately(self) -> Stepped {
        let items = self.selected_trashed();
        if items.is_empty() {
            return (self, Vec::new());
        }
        self.asking(Question::DeleteForGood(items))
    }

    /// Ask whether to empty the Trash.
    pub(crate) fn empty_trash(self) -> Stepped {
        if self.location != Location::Trash || self.rows.is_empty() {
            return (self, Vec::new());
        }
        self.asking(Question::EmptyTrash)
    }

    fn asking(self, question: Question) -> Stepped {
        (Browser { question, ..self }, Vec::new())
    }

    /// The person said yes to the question that is up.
    pub(crate) fn confirm(self) -> Stepped {
        match self.question.clone() {
            Question::EmptyTrash => Browser {
                question: Question::None,
                ..self
            }
            .run(OpPlan::EmptyTrash),
            Question::DeleteForGood(items) => Browser {
                question: Question::None,
                ..self
            }
            .run(OpPlan::DeleteForGood { items }),
            Question::None | Question::Conflict(_) => (self, Vec::new()),
        }
    }

    /// The person put the question away: a taken name is answered with Stop.
    pub(crate) fn dismiss(self) -> Stepped {
        let effects = match &self.question {
            Question::Conflict(_) => vec![Effect::Answer(Resolution::once(Answer::Stop))],
            Question::None | Question::EmptyTrash | Question::DeleteForGood(_) => Vec::new(),
        };
        (
            Browser {
                question: Question::None,
                ..self
            },
            effects,
        )
    }

    /// The person answered the taken name that is up.
    pub(crate) fn answer(self, resolution: Resolution) -> Stepped {
        match self.question {
            Question::Conflict(_) => (
                Browser {
                    question: Question::None,
                    ..self
                },
                vec![Effect::Answer(resolution)],
            ),
            Question::None | Question::EmptyTrash | Question::DeleteForGood(_) => {
                (self, Vec::new())
            }
        }
    }

    /// Remember the selection for a paste.
    pub(crate) fn copy(self) -> Stepped {
        let items = self.selected_items();
        if items.is_empty() {
            return (self, Vec::new());
        }
        (
            Browser {
                clipboard: items,
                ..self
            },
            Vec::new(),
        )
    }

    /// Copy what was copied into this folder.
    pub(crate) fn paste(self) -> Stepped {
        let Some(destination) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        if self.clipboard.is_empty() {
            return (self, Vec::new());
        }
        let sources = self.clipboard.clone();
        self.run(OpPlan::Copy(Transfer::new(sources, destination)))
    }

    /// Move what was copied into this folder; it is not on the clipboard afterwards.
    pub(crate) fn paste_move(self) -> Stepped {
        let Some(destination) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        if self.clipboard.is_empty() {
            return (self, Vec::new());
        }
        let sources = self.clipboard.clone();
        let browser = Browser {
            clipboard: Vec::new(),
            ..self
        };
        browser.run(OpPlan::Move(Transfer::new(sources, destination)))
    }

    /// Items were dropped on the folder `onto` of this one, or on this folder when `onto` is
    /// `None`.
    pub(crate) fn dropped(
        self,
        sources: Vec<ItemRef>,
        onto: Option<FileName>,
        how: DropHow,
    ) -> Stepped {
        let Some(here) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        let destination = match onto.and_then(|name| self.folder_row(&name)) {
            Some(name) => here.join(&name),
            None => here,
        };
        let sources: Vec<ItemRef> = match how {
            DropHow::Copy => sources,
            // Moving an item to the folder it is in changes nothing; a folder onto itself is the
            // executor's to refuse.
            DropHow::Natural => sources
                .into_iter()
                .filter(|item| item.folder != destination)
                .collect(),
        };
        if sources.is_empty() {
            return (self, Vec::new());
        }
        let transfer = Transfer::new(sources, destination);
        match how {
            DropHow::Natural => self.run(OpPlan::Drag(transfer)),
            DropHow::Copy => self.run(OpPlan::Copy(transfer)),
        }
    }

    /// `name`, when it is a row that opens as a folder.
    fn folder_row(&self, name: &FileName) -> Option<FileName> {
        self.rows
            .iter()
            .find(|row| row.name == *name && opens(row) == Opens::Folder)
            .map(|row| row.name.clone())
    }

    /// Take back the newest change that can be taken back.
    pub(crate) fn undo(self) -> Stepped {
        match self.undo {
            Some(_) => (self, vec![Effect::Undo]),
            None => (self, Vec::new()),
        }
    }

    /// Stop the change under way.
    pub(crate) fn cancel_work(self) -> Stepped {
        match self.work {
            Work::Running { .. } => (self, vec![Effect::Cancel]),
            Work::Idle => (self, Vec::new()),
        }
    }

    /// A worker's report.
    pub(crate) fn heard(self, event: OpEvent) -> Stepped {
        match event {
            OpEvent::Started(summary) => (
                Browser {
                    work: Work::Running {
                        summary,
                        progress: Default::default(),
                    },
                    ..self
                },
                Vec::new(),
            ),
            OpEvent::Progress(progress) => {
                let work = match self.work {
                    Work::Running { summary, .. } => Work::Running { summary, progress },
                    Work::Idle => Work::Idle,
                };
                (Browser { work, ..self }, Vec::new())
            }
            OpEvent::Asking(conflict) => (
                Browser {
                    question: Question::Conflict(conflict),
                    ..self
                },
                Vec::new(),
            ),
            OpEvent::Finished(outcome) => {
                let effects = match outcome.end {
                    End::Failed(failure) => vec![Effect::Tell(failure)],
                    End::Done if outcome.summary.kind == OpKind::Trash => {
                        vec![Effect::Announce(outcome.summary.clone())]
                    }
                    End::Done | End::Stopped => Vec::new(),
                };
                let question = match self.question {
                    Question::Conflict(_) => Question::None,
                    ref kept @ (Question::None
                    | Question::EmptyTrash
                    | Question::DeleteForGood(_)) => kept.clone(),
                };
                let starting = match outcome.end {
                    End::Done if outcome.summary.kind == OpKind::NewFolder => self.starting,
                    End::Done | End::Stopped | End::Failed(_) => None,
                };
                let browser = Browser {
                    work: Work::Idle,
                    question,
                    starting,
                    undo: outcome.undo,
                    ..self
                };
                (browser, effects)
            }
        }
    }

    /// Open the name of the folder just made, once it is among the rows.
    pub(crate) fn started(
        starting: Option<FileName>,
        rows: &[files_core::Entry],
        selection: Selection,
        rename: Rename,
    ) -> (Option<FileName>, Selection, Rename) {
        match starting {
            Some(name) if rows.iter().any(|row| row.name == name) => {
                (None, Selection::only(&name), Rename::begin(name))
            }
            other => (other, selection, rename),
        }
    }
}
