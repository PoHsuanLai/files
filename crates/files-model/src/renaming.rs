//! Renaming, as the browser steps it.

use crate::browser::Browser;
use crate::effect::Effect;
use crate::error::RenameProblem;
use crate::rename::{Decision, Rename, decide};

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// Open the name field on the one selected entry; with none or several selected, or in a
    /// place that isn't a folder, nothing happens.
    pub(crate) fn begin_rename(self) -> Stepped {
        let Some(name) = self.selection.single().cloned() else {
            return (self, Vec::new());
        };
        if self.folder().is_none() {
            return (self, Vec::new());
        }
        let rename = Rename::begin(name);
        (Browser { rename, ..self }, Vec::new())
    }

    /// The field's text changed; an earlier complaint no longer applies.
    pub(crate) fn edit_draft(self, text: String) -> Stepped {
        let rename = match self.rename {
            Rename::Editing { name, .. } => Rename::Editing {
                name,
                draft: text,
                problem: None,
            },
            Rename::Idle => Rename::Idle,
        };
        (Browser { rename, ..self }, Vec::new())
    }

    /// Stop renaming; nothing changes on disk.
    pub(crate) fn cancel_rename(self) -> Stepped {
        (
            Browser {
                rename: Rename::Idle,
                ..self
            },
            Vec::new(),
        )
    }

    /// Try to finish: an unchanged name just closes the field, a good new one is asked of the
    /// caller, and a bad or taken one keeps the field open with the reason.
    pub(crate) fn commit_rename(self) -> Stepped {
        let (name, draft) = match &self.rename {
            Rename::Editing { name, draft, .. } => (name.clone(), draft.clone()),
            Rename::Idle => return (self, Vec::new()),
        };
        let Some(folder) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        match decide(&name, &draft, self.listing.entries()) {
            Decision::Keep => self.cancel_rename(),
            Decision::Change(to) => {
                let effect = Effect::Rename {
                    folder,
                    from: name,
                    to,
                };
                (self.cancel_rename().0, vec![effect])
            }
            Decision::Refuse(problem) => (self.refuse(name, draft, problem), Vec::new()),
        }
    }

    fn refuse(self, name: files_core::FileName, draft: String, problem: RenameProblem) -> Browser {
        Browser {
            rename: Rename::Editing {
                name,
                draft,
                problem: Some(problem),
            },
            ..self
        }
    }
}
