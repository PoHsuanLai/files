//! The names of tabs, and the effects tagged with them.

use crate::browser::Browser;
use crate::effect::Effect;

/// Names one tab for as long as the window lives; an id is never used for another tab.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct TabId(pub(super) u64);

impl TabId {
    /// The first tab of a window.
    pub const FIRST: TabId = TabId(0);

    /// The id as a number, for a caller that needs a key.
    pub fn number(self) -> u64 {
        self.0
    }
}

/// An effect and the tab that asked for it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Tabbed {
    /// The tab to send the answer back to.
    pub tab: TabId,
    /// What to carry out.
    pub effect: Effect,
}

impl Tabbed {
    /// `effect`, asked for by `tab`.
    pub fn new(tab: TabId, effect: Effect) -> Tabbed {
        Tabbed { tab, effect }
    }

    /// Each of `effects`, asked for by `tab`.
    pub fn all(tab: TabId, effects: Vec<Effect>) -> Vec<Tabbed> {
        effects
            .into_iter()
            .map(|effect| Tabbed::new(tab, effect))
            .collect()
    }
}

/// One tab: its id and its browser.
#[derive(Debug, Clone, PartialEq)]
pub struct Tab {
    pub(super) id: TabId,
    pub(super) browser: Browser,
}

impl Tab {
    /// The tab's id.
    pub fn id(&self) -> TabId {
        self.id
    }

    /// The tab's browser.
    pub fn browser(&self) -> &Browser {
        &self.browser
    }
}
