//! What is selected, where the next Shift-click starts, and where the keyboard is.

use std::collections::BTreeSet;

use files_core::{Entry, FileName};

use crate::input::{Direction, Modifier};

/// The selected entries by name, the anchor a Shift-extend grows from, and the keyboard cursor.
///
/// Names, not positions, so a selection survives sorting and live changes.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Selection {
    names: BTreeSet<FileName>,
    anchor: Option<FileName>,
    cursor: Option<FileName>,
}

fn position(rows: &[Entry], name: &FileName) -> Option<usize> {
    rows.iter().position(|e| e.name == *name)
}

impl Selection {
    /// Just `name`, which is also the anchor and the cursor.
    pub fn only(name: &FileName) -> Selection {
        Selection {
            names: BTreeSet::from([name.clone()]),
            anchor: Some(name.clone()),
            cursor: Some(name.clone()),
        }
    }

    /// Whether `name` is selected.
    pub fn contains(&self, name: &FileName) -> bool {
        self.names.contains(name)
    }

    /// The selected names, in name order.
    pub fn names(&self) -> impl Iterator<Item = &FileName> {
        self.names.iter()
    }

    /// How many are selected.
    pub fn len(&self) -> usize {
        self.names.len()
    }

    /// Whether nothing is selected.
    pub fn is_empty(&self) -> bool {
        self.names.is_empty()
    }

    /// The selected name, when exactly one is.
    pub fn single(&self) -> Option<&FileName> {
        let mut names = self.names.iter();
        match (names.next(), names.next()) {
            (Some(only), None) => Some(only),
            _ => None,
        }
    }

    /// The row the keyboard is on.
    pub fn cursor(&self) -> Option<&FileName> {
        self.cursor.as_ref()
    }

    /// The row a Shift-extend grows from.
    pub fn anchor(&self) -> Option<&FileName> {
        self.anchor.as_ref()
    }

    /// The selection after a click on `name` in `rows`. A name that isn't shown changes nothing.
    pub fn click(self, rows: &[Entry], name: &FileName, modifier: Modifier) -> Selection {
        let Some(at) = position(rows, name) else {
            return self;
        };
        match modifier {
            Modifier::None => Selection::only(name),
            Modifier::Toggle => self.toggled(name),
            Modifier::Extend => self.extended_to(rows, at),
        }
    }

    /// Every row selected; the anchor and cursor stay where they were.
    pub fn select_all(self, rows: &[Entry]) -> Selection {
        Selection {
            names: rows.iter().map(|e| e.name.clone()).collect(),
            ..self
        }
    }

    /// The selection after an arrow or jump key. Plain moves the selection with the cursor, Shift
    /// grows it, and the primary key moves only the cursor.
    pub fn moved(self, rows: &[Entry], direction: Direction, modifier: Modifier) -> Selection {
        let Some(last) = rows.len().checked_sub(1) else {
            return self;
        };
        let current = self.cursor.as_ref().and_then(|c| position(rows, c));
        let target = match (direction, current) {
            (Direction::Home, _) => 0,
            (Direction::End, _) => last,
            (Direction::Down, None) => 0,
            (Direction::Up, None) => last,
            (Direction::Down, Some(at)) => (at + 1).min(last),
            (Direction::Up, Some(at)) => at.saturating_sub(1),
        };
        let Some(row) = rows.get(target) else {
            return self;
        };
        match modifier {
            Modifier::Toggle => Selection {
                cursor: Some(row.name.clone()),
                ..self
            },
            Modifier::None | Modifier::Extend => self.click(rows, &row.name, modifier),
        }
    }

    /// The selection with the names that are no longer in `rows` dropped.
    pub fn kept_in(self, rows: &[Entry]) -> Selection {
        let present = |name: &FileName| position(rows, name).is_some();
        Selection {
            names: self.names.into_iter().filter(|n| present(n)).collect(),
            anchor: self.anchor.filter(present),
            cursor: self.cursor.filter(present),
        }
    }

    fn toggled(mut self, name: &FileName) -> Selection {
        if !self.names.remove(name) {
            self.names.insert(name.clone());
        }
        Selection {
            anchor: Some(name.clone()),
            cursor: Some(name.clone()),
            ..self
        }
    }

    fn extended_to(self, rows: &[Entry], at: usize) -> Selection {
        let from = self
            .anchor
            .as_ref()
            .and_then(|a| position(rows, a))
            .unwrap_or(at);
        let span = rows
            .get(from.min(at)..=from.max(at))
            .unwrap_or_default();
        Selection {
            names: span.iter().map(|e| e.name.clone()).collect(),
            anchor: rows.get(from).map(|e| e.name.clone()),
            cursor: rows.get(at).map(|e| e.name.clone()),
        }
    }
}
