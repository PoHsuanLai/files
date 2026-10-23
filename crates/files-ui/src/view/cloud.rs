//! Cloud drives in the folder pane: the small badge after a row's name, the state of a drive that
//! cannot show its files, and the status line under a drive's folder. Motion is quire's: the ring
//! follows its share over the move token, and the glyph fades with the symbol effect when the
//! item no longer needs a badge.

use dioxus::prelude::*;
use ds::components::overlays::empty_state::EmptyForm;
use ds::prelude::{
    Button, EmptyState, Fraction, Progress, ProgressIndicator, ProgressStyle, Shown, StatusBar,
    Symbol, SymbolEffect, TransitionEffect,
};
use ds::style::tokens::control_size::ControlSize;
use files_core::CloudDrive;

use crate::cloud::{Badge, Mark};
use crate::state::Browse;
use crate::words::{SIGN_IN_BUTTON, status_words, trouble_title};

/// The badge of one row. A glyph that is no longer needed (a download that finished) fades out
/// where it stood instead of vanishing; a row that never had one draws nothing.
#[component]
pub(super) fn CloudBadge(badge: Badge) -> Element {
    // The last glyph shown: what fades out when the badge goes. Not reactive, like quire's own
    // `ds-busy-seen` flag, so keeping it never schedules another render.
    let mut last = use_hook(|| CopyValue::new(None::<Mark>));
    let now = badge.mark();
    if let Some(mark) = now
        && *last.peek() != Some(mark)
    {
        last.set(Some(mark));
    }
    let Some(mark) = now.or(*last.peek()) else {
        return rsx! {};
    };
    let shown = if now.is_some() {
        Shown::Visible
    } else {
        Shown::Hidden
    };
    let effect = SymbolEffect::Transition(TransitionEffect::Disappear, shown);
    rsx! {
        span {
            role: now.map(|_| "img"),
            "aria-label": now.map(|_| mark.name),
            "aria-hidden": if now.is_none() { Some("true") } else { None },
            Symbol { icon: mark.icon, effect }
        }
        if let Some(share) = badge.share() {
            ProgressIndicator {
                style: ProgressStyle::Ring,
                progress: Progress::Known(Fraction(share.permille())),
                size: ControlSize::Mini,
            }
        }
    }
}

/// What the folder pane says instead of the files when the drive cannot show them: its own
/// words, and (when the person has to sign in) a button that opens Accounts, only when pressed.
#[component]
pub(super) fn DriveTrouble(browse: Browse, drive: CloudDrive) -> Element {
    let title = trouble_title(&drive);
    let signing_in = drive.state == files_core::DriveState::NeedsSignIn;
    let action = signing_in.then(|| {
        rsx! {
            Button {
                label: SIGN_IN_BUTTON,
                onclick: move |_| browse.open_accounts(),
            }
        }
    });
    rsx! {
        EmptyState { form: EmptyForm::Failure, title, action }
    }
}

/// The status line under a drive's folder: how many items, what is on this computer, the room.
#[component]
pub(super) fn DriveStatus(drive: CloudDrive, items: usize) -> Element {
    rsx! {
        StatusBar { text: status_words(&drive, items) }
    }
}
