//! Tabs: several browsers in one window, one in front.
//!
//! [`Tabs`] holds a non-empty row of tabs, each a [`Browser`] with a stable [`TabId`], and the
//! one in front. Like the browser it is stepped as data: [`Tabs::step`] gives an input to the front
//! tab, or opens, closes and switches tabs; it returns the new state and the effects, each
//! tagged with the tab that asked ([`Tabbed`]).
//!
//! The caller carries the tag through: when the answer to an effect arrives it goes back with
//! [`Tabs::heard`] and the tag, so a listing that was asked for by a tab in the background lands
//! in that tab, whichever is in front by then. An answer for a tab that has since been closed is
//! dropped, except that a change it was making still ends well: a question it was asked is
//! answered Stop, and a failure is told in the front tab. What is not any one tab's (the disks changing) goes to every tab with
//! [`Tabs::broadcast`].

mod closed;
mod id;
mod shared;

use files_core::{FolderPath, Location};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::input::{Input, Now};

pub use id::{Tab, TabId, Tabbed};
use shared::{Shared, looking, vacated};

/// The browsers of a window and which one is in front. There is always at least one.
///
/// The front tab is kept apart and `others` holds the rest in order; `at` is where the front
/// tab stands among them all (`0` is first), so the order is `others[..at]`, the front, then
/// `others[at..]`.
#[derive(Debug, Clone, PartialEq)]
pub struct Tabs {
    front: Tab,
    others: Vec<Tab>,
    at: usize,
    next: u64,
}

type Stepped = (Tabs, Vec<Tabbed>);

impl Tabs {
    /// One tab on `location`, and the effects that load it.
    pub fn new(location: Location) -> (Tabs, Vec<Tabbed>) {
        let (browser, effects) = Browser::new(location);
        let tabs = Tabs {
            front: Tab {
                id: TabId::FIRST,
                browser,
            },
            others: Vec::new(),
            at: 0,
            next: TabId::FIRST.0 + 1,
        };
        (tabs, Tabbed::all(TabId::FIRST, effects))
    }

    /// The same tabs, each knowing the person's home folder, which `~` means in Go to Folder.
    pub fn with_home(self, home: FolderPath) -> Tabs {
        let Tabs {
            front,
            others,
            at,
            next,
        } = self;
        let homed = |tab: Tab| Tab {
            id: tab.id,
            browser: tab.browser.with_home(home.clone()),
        };
        Tabs {
            front: homed(front),
            others: others.into_iter().map(homed).collect(),
            at,
            next,
        }
    }

    /// The tab in front.
    pub fn front_tab(&self) -> &Tab {
        &self.front
    }

    /// The browser of the tab in front.
    pub fn front(&self) -> &Browser {
        &self.front.browser
    }

    /// The id of the tab in front.
    pub fn front_id(&self) -> TabId {
        self.front.id
    }

    /// How many tabs there are; at least one.
    pub fn len(&self) -> usize {
        self.others.len() + 1
    }

    /// Always `false`: a window keeps at least one tab.
    pub fn is_empty(&self) -> bool {
        false
    }

    /// Every tab in order, left to right.
    pub fn iter(&self) -> impl Iterator<Item = &Tab> {
        self.others
            .iter()
            .take(self.at)
            .chain(std::iter::once(&self.front))
            .chain(self.others.iter().skip(self.at))
    }

    /// The tab called `id`, if it is still open.
    pub fn get(&self, id: TabId) -> Option<&Tab> {
        self.iter().find(|tab| tab.id == id)
    }

    /// The same tabs with `browser` in front in place of the front one, nothing asked for: a
    /// click made in one event is taken back.
    pub fn with_front(self, browser: Browser) -> Tabs {
        Tabs {
            front: Tab {
                id: self.front.id,
                browser,
            },
            ..self
        }
    }

    /// The tabs after `input` for the front tab (or, for the tab inputs, for the window), and
    /// what the caller should do about it.
    pub fn step(self, input: Input, now: Now) -> (Tabs, Vec<Tabbed>) {
        if matches!(input, Input::NewTab) {
            return self.new_tab();
        }
        if matches!(input, Input::OpenInNewTab) {
            return self.open_in_new_tab();
        }
        if matches!(input, Input::CloseTab) {
            return self.close_front();
        }
        if matches!(input, Input::NextTab) {
            let to = (self.at + 1) % self.len();
            return self.show_at(to);
        }
        if matches!(input, Input::PreviousTab) {
            let to = (self.at + self.len() - 1) % self.len();
            return self.show_at(to);
        }
        if let Input::ShowTab(id) = input {
            let at = self.iter().position(|tab| tab.id == id);
            return match at {
                Some(to) => self.show_at(to),
                None => (self, Vec::new()),
            };
        }
        let id = self.front_id();
        self.heard(id, input, now)
    }

    /// The answer `input` to something tab `tab` asked for. It goes to that tab, in front or not;
    /// if the tab was closed since, it is dropped.
    pub fn heard(self, tab: TabId, input: Input, now: Now) -> (Tabs, Vec<Tabbed>) {
        let Tabs {
            mut front,
            mut others,
            at,
            next,
        } = self;
        let slot = if front.id == tab {
            Some(&mut front)
        } else {
            others.iter_mut().find(|other| other.id == tab)
        };
        let Some(slot) = slot else {
            let tabs = Tabs {
                front,
                others,
                at,
                next,
            };
            return tabs.closed(tab, input);
        };
        let shared = Shared::of(&slot.browser);
        let (browser, effects) = vacated(&mut slot.browser).step(input, now);
        let after = Shared::of(&browser);
        slot.browser = browser;
        let mut tabs = Tabs {
            front,
            others,
            at,
            next,
        };
        if shared != after {
            tabs = tabs.sharing(tab, after);
        }
        (tabs, Tabbed::all(tab, effects))
    }

    /// `input` for every tab (the disks changed). Each tab's effects are tagged with it.
    pub fn broadcast(self, input: &Input, now: Now) -> (Tabs, Vec<Tabbed>) {
        let ids: Vec<TabId> = self.iter().map(Tab::id).collect();
        ids.into_iter()
            .fold((self, Vec::new()), |(tabs, mut all), id| {
                let (tabs, effects) = tabs.heard(id, input.clone(), now);
                all.extend(effects);
                (tabs, all)
            })
    }

    /// A new tab on the front tab's place, next to it and in front.
    fn new_tab(self) -> Stepped {
        let here = self.front.browser.location().clone();
        self.opened(here)
    }

    /// The selected folder in a new tab, next to the front one and in front.
    fn open_in_new_tab(self) -> Stepped {
        match self.front.browser.lone_folder() {
            Some(folder) => self.opened(folder),
            None => (self, Vec::new()),
        }
    }

    fn opened(self, location: Location) -> Stepped {
        let Tabs {
            front,
            mut others,
            at,
            next,
        } = self;
        let (browser, effects) = front.browser.opened_like(location);
        let id = TabId(next);
        others.insert(at.min(others.len()), front);
        let tabs = Tabs {
            front: Tab { id, browser },
            others,
            at: at + 1,
            next: next + 1,
        };
        (tabs, Tabbed::all(id, effects))
    }

    /// Close the front tab: the tab to its right comes forward, or the one to its left when it
    /// was the last. The last tab asks for the window to close instead.
    fn close_front(self) -> Stepped {
        let Tabs {
            front,
            mut others,
            at,
            next,
        } = self;
        let id = front.id;
        let gone = vec![Tabbed::new(
            id,
            Effect::Unwatch(front.browser.location().clone()),
        )];
        if others.is_empty() {
            let tabs = Tabs {
                front,
                others,
                at,
                next,
            };
            return (tabs, vec![Tabbed::new(id, Effect::CloseWindow)]);
        }
        let to = at.min(others.len() - 1);
        let promoted = others.remove(to);
        let looks = looking(&promoted);
        let tabs = Tabs {
            front: promoted,
            others,
            at: to,
            next,
        };
        (tabs, gone.into_iter().chain(looks).collect())
    }

    /// Bring forward the tab at `to` (its place among all of them).
    fn show_at(self, to: usize) -> Stepped {
        let Tabs {
            front,
            mut others,
            at,
            next,
        } = self;
        if to == at || to > others.len() {
            let tabs = Tabs {
                front,
                others,
                at,
                next,
            };
            return (tabs, Vec::new());
        }
        let from = if to < at { to } else { to - 1 };
        let promoted = others.remove(from);
        let back = if to < at { at - 1 } else { at };
        others.insert(back.min(others.len()), front);
        let looks = looking(&promoted);
        let tabs = Tabs {
            front: promoted,
            others,
            at: to,
            next,
        };
        (tabs, looks)
    }

    /// Every tab but `from` takes on what the tabs share (the clipboard and what can be undone).
    fn sharing(mut self, from: TabId, shared: Shared) -> Tabs {
        for tab in std::iter::once(&mut self.front).chain(self.others.iter_mut()) {
            if tab.id != from {
                shared.put(&mut tab.browser);
            }
        }
        self
    }
}
