//! Cloud drives in the folder pane: the small badge after a row's name, the state of a drive that
//! cannot show its files, and the status line under a drive's folder. Motion is quire's: the pie
//! follows its share over the move token, and the badge fades out (`Fade`) when the item no longer
//! needs one.

use dioxus::prelude::*;
use ds::components::overlays::empty_state::EmptyForm;
use ds::prelude::{
    Button, EmptyState, Fade, Fraction, Progress, ProgressIndicator, ProgressStyle, Shown,
    StatusBar, Symbol, SymbolEffect,
};
use ds::style::tokens::control_size::ControlSize;
use files_core::CloudDrive;

use crate::cloud::Badge;
use crate::state::Browse;
use crate::words::{SIGN_IN_BUTTON, status_words, trouble_title};

/// The badge of one row: the glyph, or a pie that fills while the item moves. A badge that is no
/// longer needed (a download that finished) fades out where it stood, showing what it last
/// showed; a row that never had one draws nothing.
#[component]
pub(super) fn CloudBadge(badge: Badge) -> Element {
    // The last badge drawn: what fades out when the row needs none. Not reactive, like quire's own
    // `ds-busy-seen` flag, so keeping it never schedules another render.
    let mut last = use_hook(|| CopyValue::new(Badge::Clear));
    if badge != Badge::Clear && *last.peek() != badge {
        last.set(badge);
    }
    let drawn = if badge == Badge::Clear {
        *last.peek()
    } else {
        badge
    };
    let Some(mark) = drawn.mark() else {
        return rsx! {};
    };
    let shown = if badge == Badge::Clear {
        Shown::Hidden
    } else {
        Shown::Visible
    };
    rsx! {
        Fade { shown,
            span { role: "img", "aria-label": mark.name,
                if let Some(share) = drawn.share() {
                    ProgressIndicator {
                        style: ProgressStyle::Pie,
                        progress: Progress::Known(Fraction(share.permille())),
                        size: ControlSize::Mini,
                    }
                } else {
                    Symbol { icon: mark.icon, effect: SymbolEffect::NONE }
                }
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
