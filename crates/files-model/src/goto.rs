//! Go to Folder: a typed path, checked, then visited.

use std::path::PathBuf;

use files_core::{FolderPath, Location};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::error::{GoToProblem, ListFailure};

type Stepped = (Browser, Vec<Effect>);

/// The Go to Folder sheet: away, or asking for a path.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum GoTo {
    /// Not up.
    #[default]
    Closed,
    /// Up, with the path as typed.
    Asking {
        /// The text in the field.
        draft: String,
        /// Why the last try failed; gone when the text changes.
        problem: Option<GoToProblem>,
        /// The folder the last try is waiting to hear about.
        checking: Option<FolderPath>,
    },
}

/// The folder a typed `text` names: `~` stands for `home`, and anything else must be a full path.
/// Nothing is read from the disk; whether the folder is there is asked of the caller.
pub fn expand(text: &str, home: Option<&FolderPath>) -> Result<FolderPath, GoToProblem> {
    let text = text.trim();
    let path: Option<PathBuf> = match text.strip_prefix('~') {
        Some("") => home.map(|home| home.as_path().to_path_buf()),
        Some(rest) => rest
            .strip_prefix('/')
            .and_then(|tail| home.map(|home| home.as_path().join(tail))),
        None => Some(PathBuf::from(text)),
    };
    path.and_then(|path| FolderPath::new(path).ok())
        .ok_or(GoToProblem::NotFound)
}

impl Browser {
    /// The Go to Folder sheet.
    pub fn go_to(&self) -> &GoTo {
        &self.goto
    }

    /// Put up the sheet, with the folder shown as the text (home elsewhere, if the caller named
    /// one, else nothing).
    pub(crate) fn go_to_folder(self) -> Stepped {
        let shown = self.folder().or(self.home.as_ref());
        let draft = shown
            .map(|folder| folder.as_path().display().to_string())
            .unwrap_or_default();
        let goto = GoTo::Asking {
            draft,
            problem: None,
            checking: None,
        };
        let browser = Browser {
            goto,
            looking: false,
            ..self
        };
        (browser, Vec::new())
    }

    /// The text in the field changed; an earlier complaint no longer applies.
    pub(crate) fn go_to_typed(self, text: String) -> Stepped {
        let goto = match self.goto {
            GoTo::Asking { .. } => GoTo::Asking {
                draft: text,
                problem: None,
                checking: None,
            },
            GoTo::Closed => GoTo::Closed,
        };
        (Browser { goto, ..self }, Vec::new())
    }

    /// Return in the field: read the text as a path and ask whether the folder is there.
    pub(crate) fn go_to_submit(self) -> Stepped {
        let GoTo::Asking { draft, .. } = &self.goto else {
            return (self, Vec::new());
        };
        let draft = draft.clone();
        match expand(&draft, self.home.as_ref()) {
            Ok(folder) => {
                let goto = GoTo::Asking {
                    draft,
                    problem: None,
                    checking: Some(folder.clone()),
                };
                (Browser { goto, ..self }, vec![Effect::CheckFolder(folder)])
            }
            Err(problem) => {
                let goto = GoTo::Asking {
                    draft,
                    problem: Some(problem),
                    checking: None,
                };
                (Browser { goto, ..self }, Vec::new())
            }
        }
    }

    /// The answer to `Effect::CheckFolder`: go there, or say why not and stay. An answer for a
    /// path the sheet is no longer waiting on is ignored.
    pub(crate) fn folder_checked(
        self,
        folder: FolderPath,
        result: Result<(), ListFailure>,
    ) -> Stepped {
        let GoTo::Asking { draft, .. } = &self.goto else {
            return (self, Vec::new());
        };
        let waiting = matches!(
            &self.goto,
            GoTo::Asking { checking: Some(asked), .. } if *asked == folder
        );
        if !waiting {
            return (self, Vec::new());
        }
        match result {
            Ok(()) => Browser {
                goto: GoTo::Closed,
                ..self
            }
            .go(Location::Folder(folder)),
            Err(failure) => {
                let problem = match failure {
                    ListFailure::NoPermission => GoToProblem::NoPermission,
                    ListFailure::NotFound | ListFailure::NotAFolder | ListFailure::Other => {
                        GoToProblem::NotFound
                    }
                };
                let goto = GoTo::Asking {
                    draft: draft.clone(),
                    problem: Some(problem),
                    checking: None,
                };
                (Browser { goto, ..self }, Vec::new())
            }
        }
    }

    /// Put the sheet away (Escape, Cancel).
    pub(crate) fn go_to_cancel(self) -> Stepped {
        (
            Browser {
                goto: GoTo::Closed,
                ..self
            },
            Vec::new(),
        )
    }

    /// The same browser, knowing the person's home folder, which `~` means in Go to Folder.
    pub fn with_home(self, home: FolderPath) -> Browser {
        Browser {
            home: Some(home),
            ..self
        }
    }
}
