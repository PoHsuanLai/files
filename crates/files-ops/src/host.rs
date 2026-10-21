//! What the work asks of whoever runs it: where to report, whom to ask, and whether to go on.

use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};

use files_core::{Conflict, Progress, Resolution};

/// Whether the work goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Flow {
    /// Carry on.
    Go,
    /// Stop as soon as it is safe: what is done stays done.
    Stop,
}

/// The one who runs the work: it is told how far the work has come, is asked what to do about a
/// taken name, and says whether to go on.
pub trait Host {
    /// How far the work has come.
    fn progress(&mut self, progress: Progress);

    /// A name is taken: what to do. The work waits for the answer.
    fn ask(&mut self, conflict: &Conflict) -> Resolution;

    /// Whether to go on; asked between steps and between chunks of a copy.
    fn flow(&self) -> Flow;
}

/// A request to stop, which can be made from another thread.
#[derive(Debug, Clone, Default)]
pub struct CancelToken(Arc<AtomicBool>);

impl CancelToken {
    /// A token nobody has used to cancel.
    pub fn new() -> CancelToken {
        CancelToken::default()
    }

    /// Asks the work to stop.
    pub fn cancel(&self) {
        self.0.store(true, Ordering::SeqCst);
    }

    /// Forgets an earlier request, for the next piece of work.
    pub fn clear(&self) {
        self.0.store(false, Ordering::SeqCst);
    }

    /// [`Flow::Stop`] once [`CancelToken::cancel`] has been called.
    pub fn flow(&self) -> Flow {
        if self.0.load(Ordering::SeqCst) {
            Flow::Stop
        } else {
            Flow::Go
        }
    }
}
