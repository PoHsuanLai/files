//! Requests from outside the window: another program asked to show a folder or some files.
//!
//! The window owns this seam and never names the bus. The binary fills it from the desktop
//! crate: it makes a [`channel`], hands the [`Requests`] to the [`Launch`](crate::Launch) and
//! sends each bus call through the [`RequestSender`]. A test sends from a fake.

use std::sync::{Arc, Mutex, PoisonError};

use files_model::Input;
use tokio::sync::mpsc::{UnboundedReceiver, UnboundedSender, unbounded_channel};

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

/// The sending end: the binary's bus code holds it.
#[derive(Debug, Clone)]
pub struct RequestSender(UnboundedSender<Request>);

impl RequestSender {
    /// Hands `request` to the window. A closed window drops it.
    pub fn send(&self, request: Request) {
        let _ = self.0.send(request);
    }
}

/// The receiving end, in the [`Launch`](crate::Launch): the window takes it once.
#[derive(Debug, Clone, Default)]
pub struct Requests {
    inbox: Arc<Mutex<Option<UnboundedReceiver<Request>>>>,
}

impl Requests {
    /// A source that never sends: a window nobody can ask anything of.
    pub fn none() -> Requests {
        Requests::default()
    }

    /// The receiver, the first time; `None` after that or for [`Requests::none`].
    pub(crate) fn take(&self) -> Option<UnboundedReceiver<Request>> {
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
