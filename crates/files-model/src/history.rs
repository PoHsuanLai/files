//! Where the window has been.

use files_core::Location;

/// Whether a move is possible right now.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Availability {
    /// There is somewhere to go.
    Available,
    /// There is nowhere to go; the control is dimmed.
    Unavailable,
}

/// The places behind and ahead of the current one.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct History {
    back: Vec<Location>,
    forward: Vec<Location>,
}

impl History {
    /// The history after leaving `from` for somewhere new: `from` can be gone back to, and
    /// whatever lay ahead is forgotten.
    pub fn visited(&self, from: Location) -> History {
        let mut back = self.back.clone();
        back.push(from);
        History {
            back,
            forward: Vec::new(),
        }
    }

    /// The history and the place after going back from `current`, if there is a place behind.
    pub fn went_back(&self, current: &Location) -> Option<(History, Location)> {
        let mut back = self.back.clone();
        let to = back.pop()?;
        let mut forward = self.forward.clone();
        forward.push(current.clone());
        Some((History { back, forward }, to))
    }

    /// The history and the place after going forward from `current`, if there is a place ahead.
    pub fn went_forward(&self, current: &Location) -> Option<(History, Location)> {
        let mut forward = self.forward.clone();
        let to = forward.pop()?;
        let mut back = self.back.clone();
        back.push(current.clone());
        Some((History { back, forward }, to))
    }

    /// Whether going back works.
    pub fn back_availability(&self) -> Availability {
        availability(&self.back)
    }

    /// Whether going forward works.
    pub fn forward_availability(&self) -> Availability {
        availability(&self.forward)
    }
}

fn availability(places: &[Location]) -> Availability {
    if places.is_empty() {
        Availability::Unavailable
    } else {
        Availability::Available
    }
}
