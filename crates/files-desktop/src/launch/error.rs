//! Why an app's description, its command or its start was refused, in the words a person reads.

/// Why a desktop entry is not an app.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum EntryError {
    /// The text could not be read as a desktop entry.
    #[error("That isn\u{2019}t an app\u{2019}s description.")]
    Unreadable,
    /// No `[Desktop Entry]` group.
    #[error("That isn\u{2019}t an app\u{2019}s description.")]
    NoEntry,
    /// Its `Type` is not `Application`.
    #[error("\u{201c}{0}\u{201d} isn\u{2019}t an app.")]
    NotAnApp(String),
    /// It is started by a command and has none.
    #[error("\u{201c}{0}\u{201d} has no command to run.")]
    NoCommand(String),
}

/// Why an `Exec` value was refused.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum ExecError {
    /// Nothing to run.
    #[error("The app has no command to run.")]
    Empty,
    /// A quote is never closed.
    #[error("The app\u{2019}s command has a quote that is never closed.")]
    UnclosedQuote,
    /// A `%` code the specification does not have.
    #[error("The app\u{2019}s command uses an unknown code, \u{201c}%{0}\u{201d}.")]
    UnknownCode(char),
    /// A `%` at the very end.
    #[error("The app\u{2019}s command ends with a stray \u{201c}%\u{201d}.")]
    TrailingPercent,
    /// A code where the program's name should be.
    #[error("The app\u{2019}s command starts with a code instead of a program.")]
    CodeInProgram,
}

/// Why files were not opened, in the words a person reads.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum LaunchError {
    /// Nothing opens this file. Holds the file's name.
    #[error("There is no app set to open \u{201c}{0}\u{201d}.")]
    NoApp(String),
    /// The app runs in a terminal window, which files cannot open yet. Holds the app's name.
    #[error("\u{201c}{0}\u{201d} runs in a terminal window, which Files can\u{2019}t open yet.")]
    NeedsTerminal(String),
    /// The app could not be started.
    #[error("\u{201c}{app}\u{201d} couldn\u{2019}t be opened. {reason}")]
    Failed {
        /// The app's name.
        app: String,
        /// What went wrong.
        reason: String,
    },
}

/// Why something could not be started, in a few plain words.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[error("{reason}")]
pub struct StartError {
    /// What went wrong.
    pub reason: String,
}

impl StartError {
    /// An error that says `reason`.
    pub fn new(reason: impl Into<String>) -> StartError {
        StartError {
            reason: reason.into(),
        }
    }
}
