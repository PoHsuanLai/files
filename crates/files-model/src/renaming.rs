//! Renaming, as the browser steps it.

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

use files_core::{Asking, Entry, FileName, FolderPath};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::error::RenameProblem;
use crate::open::{Opens, opens};
use crate::ops::{ExtensionChange, Question};
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
            Decision::Change(to) => self.changing(folder, name, to),
            Decision::Refuse(problem) => (self.refuse(name, draft, problem), Vec::new()),
        }
    }

    /// Rename `from` to `to`, after asking first when that changes a file's extension and the
    /// person has not chosen to go straight on.
    fn changing(self, folder: FolderPath, from: FileName, to: FileName) -> Stepped {
        let entries = self.listing.entries();
        let a_file = entries
            .iter()
            .find(|entry| entry.name == from)
            .is_some_and(|entry| opens(entry) != Opens::Folder);
        let asks = self.behaviour.changing_extension == Asking::Ask
            && a_file
            && lowered_extension(&from) != lowered_extension(&to);
        if !asks {
            let effect = Effect::Rename { folder, from, to };
            return (self.cancel_rename().0, vec![effect]);
        }
        let kept = keeping_extension(&from, &to, entries);
        let change = ExtensionChange::new(folder, from, to, kept);
        self.cancel_rename()
            .0
            .asking(Question::ChangeExtension(change))
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

/// The extension of `name` in lower case, so a change of case alone is not a change of extension.
fn lowered_extension(name: &FileName) -> Option<Vec<u8>> {
    name.extension()
        .map(|extension| extension.as_bytes().to_ascii_lowercase())
}

/// `to` with the extension `from` has put back (none, if `from` has none). The file's own name
/// when that is not a name it can have: another entry of `entries` holds it.
fn keeping_extension(from: &FileName, to: &FileName, entries: &[Entry]) -> FileName {
    let mut bytes = to.stem().as_bytes().to_vec();
    if let Some(extension) = from.extension() {
        bytes.push(b'.');
        bytes.extend_from_slice(extension.as_bytes());
    }
    FileName::new(OsString::from_vec(bytes))
        .ok()
        .filter(|kept| kept == from || !entries.iter().any(|entry| entry.name == *kept))
        .unwrap_or_else(|| from.clone())
}
