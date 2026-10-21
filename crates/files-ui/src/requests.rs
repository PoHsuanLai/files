//! Requests from outside the window: another program asked to show a folder or some files, to
//! change some, or to say what the window holds.
//!
//! The window owns this seam and never names the bus. The binary fills it from the desktop
//! crate: it makes a [`channel`], hands the [`Requests`] to the [`Launch`](crate::Launch) and
//! sends each bus call through the [`RequestSender`]. A test sends from a fake.
//!
//! Most asks are model inputs ([`Request`], or [`RequestSender::send_quietly`] when the window
//! should not come to the front), so they take the same road as the person's own and a change is
//! queued, shown and undone like theirs. Two asks are questions: [`RequestSender::ask`] sends a
//! [`Query`] and the window answers with a [`Reading`] of its state.

use std::sync::{Arc, Mutex, PoisonError};

use files_core::{ItemRef, Location};
use files_model::{Browser, Input};
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};
use tokio::sync::oneshot;

use crate::error::ReadError;

/// An activation token: what lets the window raise itself when it was not asked by a click on
/// the window.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RaiseToken(pub String);

/// One thing another program asked for, as the input the browser takes.
#[derive(Debug, Clone, PartialEq)]
pub struct Request {
    /// What the browser is to do (go to a folder, reveal entries in one).
    pub input: Input,
    /// Raise the window with this token; `None` raises it without one.
    pub raise: Option<RaiseToken>,
}

/// What another program may ask the window to tell it.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Query {
    /// The items selected in the folder shown.
    Selection,
    /// Where the window is.
    CurrentFolder,
}

/// What the window answers to a [`Query`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Reading {
    /// The selected items in shown order: none when nothing is selected, or in the Trash.
    Selection(Vec<ItemRef>),
    /// The place the window shows.
    Place(Location),
}

impl Reading {
    /// What `browser` says to `query`.
    pub fn of(browser: &Browser, query: Query) -> Reading {
        match query {
            Query::Selection => Reading::Selection(
                browser
                    .selected_targets()
                    .into_iter()
                    .map(|target| ItemRef::new(target.folder, target.name))
                    .collect(),
            ),
            Query::CurrentFolder => Reading::Place(browser.location().clone()),
        }
    }
}

/// A question on its way to the window, with the way back.
#[derive(Debug)]
pub(crate) struct Read {
    query: Query,
    reply: oneshot::Sender<Reading>,
}

impl Read {
    /// Answers from `browser`. A caller that stopped waiting is not told.
    pub(crate) fn answer(self, browser: &Browser) {
        let _ = self.reply.send(Reading::of(browser, self.query));
    }
}

/// What reaches the window.
#[derive(Debug)]
pub(crate) enum Asked {
    /// An input, and the window comes to the front.
    Do(Request),
    /// An input, and the window stays where it is.
    Quietly(Input),
    /// A question.
    Read(Read),
}

/// The answer to a question, on its way.
#[derive(Debug)]
pub struct Answer {
    inbox: oneshot::Receiver<Reading>,
}

impl Answer {
    /// Waits for the window's reading; an error when the window closed first.
    pub async fn wait(self) -> Result<Reading, ReadError> {
        self.inbox.await.map_err(|_| ReadError::Closed)
    }

    /// The reading, if it has arrived; for a caller that polls (a test driving a window).
    pub fn try_now(&mut self) -> Option<Reading> {
        self.inbox.try_recv().ok()
    }
}

/// The sending end: the binary's bus code holds it.
#[derive(Debug, Clone)]
pub struct RequestSender(UnboundedSender<Asked>);

impl RequestSender {
    /// Hands `request` to the window, which comes to the front. A closed window drops it.
    pub fn send(&self, request: Request) {
        let _ = self.0.send(Asked::Do(request));
    }

    /// Hands `input` to the window, which stays where it is. A closed window drops it.
    pub fn send_quietly(&self, input: Input) {
        let _ = self.0.send(Asked::Quietly(input));
    }

    /// Asks the window `query`; an error when the window is already closed.
    pub fn ask(&self, query: Query) -> Result<Answer, ReadError> {
        let (reply, inbox) = oneshot::channel();
        self.0
            .send(Asked::Read(Read { query, reply }))
            .map_err(|_| ReadError::Closed)?;
        Ok(Answer { inbox })
    }
}

/// The receiving end, in the [`Launch`](crate::Launch): the window takes it once.
#[derive(Debug, Clone, Default)]
pub struct Requests {
    inbox: Arc<Mutex<Option<UnboundedReceiver<Asked>>>>,
}

impl Requests {
    /// A source that never sends: a window nobody can ask anything of.
    pub fn none() -> Requests {
        Requests::default()
    }

    /// The receiver, the first time; `None` after that or for [`Requests::none`].
    pub(crate) fn take(&self) -> Option<UnboundedReceiver<Asked>> {
        self.inbox
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .take()
    }
}

/// A sender and the source it feeds. Requests sent before the window starts wait for it.
pub fn channel() -> (RequestSender, Requests) {
    let (sender, receiver) = unbounded_channel();
    let requests = Requests {
        inbox: Arc::new(Mutex::new(Some(receiver))),
    };
    (RequestSender(sender), requests)
}
