//! Typing a name to jump to it.

use std::time::Duration;

use files_core::Entry;

use crate::input::Now;

/// How long a pause ends the word being typed (design/30, TypeaheadReset).
pub const RESET: Duration = Duration::from_millis(1000);

/// The letters typed so far and when the last one came.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct TypeSelect {
    typed: String,
    last: Option<Duration>,
}

impl TypeSelect {
    /// After `c` at `now`: it extends the word when the last key came within [`RESET`], else it
    /// starts a new one.
    pub fn pushed(self, c: char, now: Now) -> TypeSelect {
        let continues = self
            .last
            .is_some_and(|last| now.0.saturating_sub(last) <= RESET);
        let mut typed = if continues { self.typed } else { String::new() };
        typed.extend(c.to_lowercase());
        TypeSelect {
            typed,
            last: Some(now.0),
        }
    }

    /// The word typed so far, lower-cased.
    pub fn prefix(&self) -> &str {
        &self.typed
    }

    /// The index of the first row whose name starts with the word, ignoring case.
    pub fn find(&self, rows: &[Entry]) -> Option<usize> {
        rows.iter()
            .position(|e| e.name.display().to_lowercase().starts_with(&self.typed))
    }
}
