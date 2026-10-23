//! The browser in a signal, and the loop that runs its effects through the edge.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use dioxus::prelude::*;
use ds::prelude::use_toasts;
use ds::stack::toast_hub::UndoToken;
use ds_blitz::{WindowHandle, use_window_handle};
use files_model::{Browser, Input, Rename, TabId, Tabs};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::edge::{self, AppChoiceView, Edge, Heard};
use crate::icons::Icons;
use crate::launch::Launch;
use crate::looks::Looks;
use crate::preferences::Preferences;
use crate::requests::Asked;
use crate::thumbs::Thumbs;
use crate::words::announce_words;

/// The browser in front, read through the tabs: what the views draw and ask.
pub(crate) type Front = MappedSignal<Browser, Signal<Tabs>, fn(&Tabs) -> &Browser>;

/// One window's tabs. `Copy`: hand it to any handler. The views read [`Browse::state`] (the browser
/// in front) and [`Browse::tabs`]; every change goes through [`Browse::send`].
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Browse {
    /// The tabs. Read them; change them only with [`Browse::send`] and its kin.
    pub(crate) tabs: Signal<Tabs>,
    /// The browser in front.
    pub(crate) state: Front,
    window: CopyValue<Option<WindowHandle>>,
    edge: CopyValue<Rc<Edge>>,
    start: Instant,
    /// The previews the peek worker has made, for Quick Look and the preview pane.
    pub(crate) looked: Signal<Looks>,
    /// The thumbnails the icon view has been given.
    pub(crate) thumbs: Signal<Thumbs>,
    /// The person's choices, as the settings file last said.
    pub(crate) preferences: Signal<Preferences>,
    /// Which icon each item gets.
    pub(crate) icons: Signal<Icons>,
}

impl std::fmt::Debug for Browse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Browse")
    }
}

/// What the first render makes once: the browser, the edge and the end of its channel.
#[derive(Clone)]
struct Setup {
    first: Tabs,
    edge: Rc<Edge>,
    inbox: Rc<RefCell<Option<UnboundedReceiver<Heard>>>>,
    start: Instant,
}

impl Browse {
    /// Steps the tab in front with `input` and carries out the effects it asks for. The tab inputs
    /// (new, close, switch) are the window's.
    pub(crate) fn send(&self, input: Input) {
        let mut tabs = self.tabs;
        let current = Tabs::clone(&tabs.peek());
        let (next, effects) = current.step(input, edge::now(self.start));
        tabs.set(next);
        self.edge.read().run(effects);
    }

    /// Gives `input` to the tab `tab`, the one that asked for it, whichever is in front now.
    fn send_to(&self, tab: TabId, input: Input) {
        let mut tabs = self.tabs;
        let current = Tabs::clone(&tabs.peek());
        let (next, effects) = current.heard(tab, input, edge::now(self.start));
        tabs.set(next);
        self.edge.read().run(effects);
    }

    /// Gives `input` to every tab.
    fn send_to_all(&self, input: &Input) {
        let mut tabs = self.tabs;
        let current = Tabs::clone(&tabs.peek());
        let (next, effects) = current.broadcast(input, edge::now(self.start));
        tabs.set(next);
        self.edge.read().run(effects);
    }

    /// Closes the window.
    fn close_window(&self) {
        let window = self.window.read();
        if let Some(window) = &*window {
            window.close();
        }
    }

    /// The apps that open every selected item, for Open With.
    pub(crate) fn apps_for_selection(&self) -> Vec<AppChoiceView> {
        let targets = self.state.read().selected_targets();
        self.edge.read().apps_for(&targets)
    }

    /// Every installed app.
    pub(crate) fn all_apps(&self) -> Vec<AppChoiceView> {
        self.edge.read().all_apps()
    }

    /// Opens Accounts in Settings (the button of a drive that needs signing in).
    pub(crate) fn open_accounts(&self) {
        self.edge.read().open_accounts();
    }

    /// The edge, for a tile that wants a thumbnail while it is drawn and says so when it goes.
    pub(crate) fn edge(&self) -> Rc<Edge> {
        Rc::clone(&self.edge.read())
    }

    /// Whether a name is being typed in place.
    pub(crate) fn renaming(&self) -> bool {
        matches!(self.state.read().rename(), Rename::Editing { .. })
    }
}

/// The browser of this window, started on `launch`'s place, and the loop that carries out its
/// effects and feeds the answers back. Call it once, in a component under `Ds` (a failure is
/// told as a toast).
pub(crate) fn use_browse(launch: &Launch) -> Browse {
    let setup = use_hook(|| {
        let (first, effects) = Tabs::new(launch.start.clone());
        let first = first.with_behaviour(launch.preferences.borrow().behaviour);
        let first = match &launch.home {
            Some(home) => first.with_home(home.clone()),
            None => first,
        };
        let (carrier, inbox) = Edge::new(launch.seams.clone());
        carrier.run(effects);
        Setup {
            first,
            edge: Rc::new(carrier),
            inbox: Rc::new(RefCell::new(Some(inbox))),
            start: edge::start(),
        }
    });
    let tabs = use_signal(|| setup.first.clone());
    let looked = use_signal(Looks::default);
    let thumbs = use_signal(Thumbs::default);
    let preferences = use_signal(|| launch.preferences.borrow().clone());
    let window = use_window_handle();
    let browse = Browse {
        tabs,
        state: tabs.map(Tabs::front as fn(&Tabs) -> &Browser),
        window: use_hook(|| CopyValue::new(window.clone())),
        edge: use_hook(|| CopyValue::new(Rc::clone(&setup.edge))),
        start: setup.start,
        looked,
        thumbs,
        preferences,
        icons: use_context::<Signal<Icons>>(),
    };
    let mut feed = launch.preferences.clone();
    use_hook(move || {
        // A change of the person's choices: the sidebar's rows, and every tab's behaviour.
        spawn(async move {
            while feed.changed().await.is_ok() {
                let next = feed.borrow_and_update().clone();
                let mut slot = browse.preferences;
                slot.set(next.clone());
                browse.send_to_all(&Input::Behave(next.behaviour));
            }
        })
    });
    let toasts = use_toasts();
    let requests = launch.requests.clone();
    use_hook(move || {
        spawn(async move {
            let Some(mut inbox) = requests.take() else {
                return;
            };
            while let Some(asked) = inbox.recv().await {
                match asked {
                    Asked::Do(request) => {
                        browse.send(request.input);
                        let window = browse.window.read();
                        if let Some(window) = &*window {
                            window.focus_with_token(request.raise.map(|token| token.0));
                        }
                    }
                    Asked::Quietly(input) => browse.send(input),
                    Asked::Read(read) => read.answer(&browse.state.peek()),
                }
            }
        })
    });
    use_hook(move || {
        // The toast's Undo takes back the newest change, which is the one it announced. Made
        // once, in the hook, so the handler belongs to this component, which outlives every toast.
        let on_undo = EventHandler::new(move |_: UndoToken| browse.send(Input::Undo));
        spawn(async move {
            let Some(mut inbox) = setup.inbox.borrow_mut().take() else {
                return;
            };
            let mut announced: u64 = 0;
            while let Some(heard) = inbox.recv().await {
                match heard {
                    Heard::Tab(tab, input) => browse.send_to(tab, input),
                    Heard::All(input) => browse.send_to_all(&input),
                    Heard::CloseWindow => browse.close_window(),
                    Heard::Told(text) => toasts.push(text, None),
                    Heard::Looked(item, card) => {
                        let mut slot = browse.looked;
                        slot.write().put(item, card);
                    }
                    Heard::Thumbed(key, thumbed) => {
                        let mut shelf = browse.thumbs;
                        shelf.write().put(key, thumbed);
                    }
                    Heard::Announced(summary) => {
                        announced += 1;
                        toasts.push_undoable(
                            announce_words(&summary),
                            UndoToken(announced),
                            on_undo,
                        );
                    }
                }
            }
        })
    });
    browse
}
