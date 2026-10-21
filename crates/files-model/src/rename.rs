//! Renaming one entry in place.

use files_core::{Entry, FileName, NameError};

/// Why a rename can't go through yet, in words a person reads.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum RenameProblem {
    /// The text isn't a name.
    #[error("{0}")]
    Invalid(NameError),
    /// Something in the folder already has that name.
    #[error("That name is taken.")]
    Taken,
}

/// Whether a name is being edited.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum Rename {
    /// Nothing is being renamed.
    #[default]
    Idle,
    /// The name field is open on `name`.
    Editing {
        /// The entry being renamed, by its name now.
        name: FileName,
        /// The text in the field.
        draft: String,
        /// Why the draft can't be used, after a try.
        problem: Option<RenameProblem>,
    },
}

impl Rename {
    /// The editing state for `name`, with its own name as the draft.
    pub fn begin(name: FileName) -> Rename {
        Rename::Editing {
            draft: name.display().into_owned(),
            name,
            problem: None,
        }
    }

    /// Whether the name field is open.
    pub(crate) fn is_editing(&self) -> bool {
        match self {
            Rename::Editing { .. } => true,
            Rename::Idle => false,
        }
    }

    /// The same, when `name` is still in `entries`; otherwise nothing is being renamed any more.
    pub fn kept_in(self, entries: &[Entry]) -> Rename {
        let present = match &self {
            Rename::Editing { name, .. } => entries.iter().any(|e| e.name == *name),
            Rename::Idle => false,
        };
        if present { self } else { Rename::Idle }
    }
}

/// What committing a draft does.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) enum Decision {
    /// The name is the same: nothing to do.
    Keep,
    /// Rename to this.
    Change(FileName),
    /// Not yet.
    Refuse(RenameProblem),
}

/// What committing `draft` as the new name of `name` does, given the folder's `entries`.
pub(crate) fn decide(name: &FileName, draft: &str, entries: &[Entry]) -> Decision {
    if draft == name.display() {
        return Decision::Keep;
    }
    match FileName::new(draft) {
        Err(problem) => Decision::Refuse(RenameProblem::Invalid(problem)),
        Ok(to) if to == *name => Decision::Keep,
        Ok(to) if entries.iter().any(|e| e.name == to) => Decision::Refuse(RenameProblem::Taken),
        Ok(to) => Decision::Change(to),
    }
}
