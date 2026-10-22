//! What the tabs of a window share, and the small moves between tabs.

use files_core::{ItemRef, Location, Summary};

use super::{Effect, Tab, Tabbed, Tabs};
use crate::browser::Browser;
use crate::preview::Pane;

/// What is the window's, not one tab's: what was copied, what Undo would take back and whether
/// the preview pane is up.
#[derive(Debug, Clone, PartialEq)]
pub(super) struct Shared {
    clipboard: Vec<ItemRef>,
    undo: Option<files_core::Summary>,
    pane: Pane,
}

impl Shared {
    pub(super) fn of(browser: &Browser) -> Shared {
        Shared {
            clipboard: browser.clipboard.clone(),
            undo: browser.undo.clone(),
            pane: browser.pane,
        }
    }

    pub(super) fn put(&self, browser: &mut Browser) {
        browser.clipboard = self.clipboard.clone();
        browser.undo = self.undo.clone();
        browser.pane = self.pane;
    }
}

/// The asks for a tab that has just come to the front with Quick Look or the preview pane up: the
/// preview shown is the last one asked for, which may be of another tab's item, and a pane that
/// was put up while the tab was behind has not read its item's facts.
pub(super) fn looking(tab: &Tab) -> Vec<Tabbed> {
    let quick = tab.browser.quick_item().map(Effect::Look);
    let pane = tab
        .browser
        .previewed_item()
        .into_iter()
        .flat_map(|item| [Effect::Look(item.clone()), Effect::ReadDetails(item)]);
    quick
        .into_iter()
        .chain(pane)
        .map(|effect| Tabbed::new(tab.id, effect))
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
