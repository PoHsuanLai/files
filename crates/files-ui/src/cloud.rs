//! What a cloud drive and its items look like: the pure mapping from the model's states to the
//! small marks quire draws (glyphs only, no words in the row).

use ds::prelude::Icon;
use files_core::{CloudItem, CloudKind, CloudState, DriveState, Share};

/// A small mark: a glyph and the words that name it for assistive technology.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Mark {
    /// The glyph.
    pub icon: Icon,
    /// What it says.
    pub name: &'static str,
}

impl Mark {
    const fn new(icon: Icon, name: &'static str) -> Mark {
        Mark { icon, name }
    }
}

/// What a row of a cloud drive shows after its name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Badge {
    /// Nothing: not in a drive, not read yet, or downloaded (as Finder shows it).
    Clear,
    /// A glyph.
    Marked(Mark),
    /// A glyph and how far the transfer is.
    Moving(Mark, Share),
}

impl Badge {
    /// The glyph, when there is one.
    pub fn mark(self) -> Option<Mark> {
        match self {
            Badge::Clear => None,
            Badge::Marked(mark) | Badge::Moving(mark, _) => Some(mark),
        }
    }

    /// How far the transfer is, while one moves.
    pub fn share(self) -> Option<Share> {
        match self {
            Badge::Moving(_, share) => Some(share),
            Badge::Clear | Badge::Marked(_) => None,
        }
    }
}

const IN_THE_CLOUD: Mark = Mark::new(Icon::Download, "In the cloud");
const DOWNLOADING: Mark = Mark::new(Icon::Download, "Downloading");
const UPLOADING: Mark = Mark::new(Icon::Upload, "Uploading");
const WAITING: Mark = Mark::new(Icon::Clock, "Waiting to upload");
const FAILED: Mark = Mark::new(Icon::TriangleAlert, "Couldn\u{2019}t sync");
const CONFLICT: Mark = Mark::new(Icon::OctagonAlert, "Two versions");
const ONLINE: Mark = Mark::new(Icon::Globe, "Opens in your browser");

/// The badge of an item whose details are `item` (`None` before they are read).
pub fn badge_of(item: Option<&CloudItem>) -> Badge {
    let Some(item) = item else {
        return Badge::Clear;
    };
    if item.kind == CloudKind::Online {
        return Badge::Marked(ONLINE);
    }
    let moving = |mark: Mark| match item.share {
        Some(share) => Badge::Moving(mark, share),
        None => Badge::Marked(mark),
    };
    match item.state {
        CloudState::Cloud => Badge::Marked(IN_THE_CLOUD),
        CloudState::Downloading => moving(DOWNLOADING),
        CloudState::Uploading => moving(UPLOADING),
        CloudState::Waiting => Badge::Marked(WAITING),
        CloudState::Error => Badge::Marked(FAILED),
        CloudState::Conflict => Badge::Marked(CONFLICT),
        CloudState::Downloaded | _ => Badge::Clear,
    }
}

/// The mark after a drive's name in the sidebar; none while it is up to date.
pub fn drive_mark(state: DriveState) -> Option<Mark> {
    match state {
        DriveState::Ready => None,
        DriveState::Starting | DriveState::Syncing => Some(Mark::new(Icon::Refresh, "Syncing")),
        DriveState::Offline => Some(Mark::new(Icon::WifiOff, "Offline")),
        DriveState::Paused => Some(Mark::new(Icon::Pause, "Syncing paused")),
        DriveState::NeedsSignIn => Some(Mark::new(Icon::Key, "Needs sign-in")),
        DriveState::Error | _ => Some(Mark::new(Icon::TriangleAlert, "Something went wrong")),
    }
}
