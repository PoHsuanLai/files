//! What becomes of a report for a tab that has been closed since it asked.

use files_core::{Answer, Resolution};

use super::{Effect, Stepped, TabId, Tabbed, Tabs};
use crate::input::Input;
use crate::ops::{End, OpEvent};

impl Tabs {
    /// `input` was the answer to something `tab` asked before it was closed. Nobody is left to
    /// show it, so most of it is dropped. But the queue of changes is waiting on its jobs:
    /// a conflict question is answered Stop, so the queue goes on; a failure is told as a toast
    /// in the front tab; and what Undo would take back is updated in every tab.
    pub(super) fn closed(self, tab: TabId, input: Input) -> Stepped {
        let Input::Op(event) = input else {
            return (self, Vec::new());
        };
        match event {
            OpEvent::Asking(_) => {
                let stop = Effect::Answer(Resolution::once(Answer::Stop));
                (self, vec![Tabbed::new(tab, stop)])
            }
            OpEvent::Finished(outcome) => {
                let told = match outcome.end {
                    End::Failed(failure) => {
                        vec![Tabbed::new(self.front_id(), Effect::Tell(failure))]
                    }
                    End::Done | End::Stopped => Vec::new(),
                };
                (self.undoable(outcome.undo), told)
            }
            OpEvent::Started(_) | OpEvent::Progress(_) => (self, Vec::new()),
        }
    }
}
