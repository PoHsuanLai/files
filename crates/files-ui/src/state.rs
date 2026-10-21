//! The browser in a signal, and the loop that runs its effects through the edge.

use std::cell::RefCell;
use std::rc::Rc;
use std::time::Instant;

use dioxus::prelude::*;
use ds::prelude::use_toasts;
use ds::stack::toast_hub::UndoToken;
use ds_blitz::use_window_handle;
use files_model::{Browser, Input, Rename};
use tokio::sync::mpsc::UnboundedReceiver;

use crate::edge::{self, Edge, Heard};
use crate::launch::Launch;
use crate::words::announce_words;

/// One window's browser. `Copy`: hand it to any handler. The views read [`Browse::state`]; every
/// change goes through [`Browse::send`].
#[derive(Clone, Copy, PartialEq)]
pub(crate) struct Browse {
    /// The browser. Read it; change it only with [`Browse::send`].
    pub(crate) state: Signal<Browser>,
    edge: CopyValue<Rc<Edge>>,
    start: Instant,
    /// The browser as it was just before the last click, for the length of one event: a key the
    /// table reads as "move the cursor" (Command+Down) and the window reads as "open" arrive in
    /// that order, and the open is of the row the cursor was on.
    trail: Signal<Option<Browser>>,
}

impl std::fmt::Debug for Browse {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Browse")
    }
}

/// What the first render makes once: the browser, the edge and the end of its channel.
#[derive(Clone)]
struct Setup {
    first: Browser,
    edge: Rc<Edge>,
    inbox: Rc<RefCell<Option<UnboundedReceiver<Heard>>>>,
    start: Instant,
}

impl Browse {
    /// Steps the browser with `input` and carries out the effects it asks for.
    pub(crate) fn send(&self, input: Input) {
        let mut state = self.state;
        let current = Browser::clone(&state.peek());
        let before = matches!(input, Input::Click(..)).then(|| current.clone());
        let (next, effects) = current.step(input, edge::now(self.start));
        if let Some(before) = before {
            let mut trail = self.trail;
            trail.set(Some(before));
            // Gone by the next turn of the loop, which is after every handler of this event.
            spawn(async move { trail.set(None) });
        }
        state.set(next);
        self.edge.read().run(effects);
    }

    /// What the browser was before this event's click, once; `None` if no click came first.
    pub(crate) fn take_trail(&self) -> Option<Browser> {
        let mut trail = self.trail;
        std::mem::take(&mut *trail.write())
    }

    /// Puts the browser back as it was, without asking for anything: a click this event made is
    /// undone.
    pub(crate) fn restore(&self, before: Browser) {
        let mut state = self.state;
        state.set(before);
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
        let (first, effects) = Browser::new(launch.start.clone());
        let (carrier, inbox) = Edge::new(launch.seams.clone());
        carrier.run(effects);
        Setup {
            first,
            edge: Rc::new(carrier),
            inbox: Rc::new(RefCell::new(Some(inbox))),
            start: edge::start(),
        }
    });
    let state = use_signal(|| setup.first.clone());
    let trail = use_signal(|| None);
    let browse = Browse {
        state,
        edge: use_hook(|| CopyValue::new(Rc::clone(&setup.edge))),
        start: setup.start,
        trail,
    };
    let toasts = use_toasts();
    let window = use_window_handle();
    let requests = launch.requests.clone();
    use_hook(move || {
        spawn(async move {
            let Some(mut asked) = requests.take() else {
                return;
            };
            while let Some(request) = asked.recv().await {
                browse.send(request.input);
                if let Some(window) = &window {
                    window.focus_with_token(request.raise.map(|token| token.0));
                }
            }
        })
    });
    // The toast's Undo takes back the newest change, which is the one it announced. The handler
    // belongs to this component, which outlives every toast.
    let on_undo = EventHandler::new(move |_: UndoToken| browse.send(Input::Undo));
    use_hook(move || {
        spawn(async move {
            let Some(mut inbox) = setup.inbox.borrow_mut().take() else {
                return;
            };
            let mut announced: u64 = 0;
            while let Some(heard) = inbox.recv().await {
                match heard {
                    Heard::Input(input) => browse.send(input),
                    Heard::Told(text) => toasts.push(text, None),
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
