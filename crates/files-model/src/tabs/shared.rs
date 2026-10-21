//! What the tabs of a window share, and the small moves between tabs.

use files_core::{ItemRef, Location, Summary};

use super::{Effect, Tab, Tabbed, Tabs};
use crate::browser::Browser;

/// What is the window's, not one tab's: what was copied and what Undo would take back.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Shared {
    clipboard: Vec<ItemRef>,
    undo: Option<files_core::Summary>,
}

impl Shared {
    pub(super) fn of(browser: &Browser) -> Shared {
        Shared {
            clipboard: browser.clipboard.clone(),
            undo: browser.undo.clone(),
        }
    }

    pub(super) fn put(&self, browser: &mut Browser) {
        browser.clipboard = self.clipboard.clone();
        browser.undo = self.undo.clone();
    }
}

/// Quick Look's ask for a tab that has just come to the front with it up: the preview shown is
/// the last one asked for, which may be of another tab's item.
pub(super) fn looking(tab: &Tab) -> Vec<Tabbed> {
    tab.browser
        .quick_item()
        .map(|item| Tabbed::new(tab.id, Effect::Look(item)))
        .into_iter()
        .collect()
}

/// Takes the browser out of `slot` to be stepped, leaving an empty one that is put over at once.
pub(super) fn vacated(slot: &mut Browser) -> Browser {
    let (empty, _) = Browser::new(Location::Trash);
    std::mem::replace(slot, empty)
}

impl Tabs {
    /// Every tab takes on `undo` as what Undo would take back.
    pub(super) fn undoable(mut self, undo: Option<Summary>) -> Tabs {
        for tab in std::iter::once(&mut self.front).chain(self.others.iter_mut()) {
            tab.browser.undo = undo.clone();
        }
        self
    }
}
