//! The questions the window asks before it goes on: a taken name, and erasing for good. Each is
//! quire's `Alert`, with Finder's words.

use dioxus::prelude::*;
use ds::components::overlays::alert_model::{AlertButton, AlertRole, AlertStyle, Suppression};
use ds::prelude::{Alert, Check, TextLine};
use files_core::{Answer, Conflict, Resolution};
use files_model::{Input, Question};

use crate::state::Browse;
use crate::words::{CANNOT_UNDO, conflict_words, erase_words};

/// The question that is up, if any.
#[component]
pub(super) fn Questions(browse: Browse) -> Element {
    let question = browse.state.read().question().clone();
    match question {
        Question::None => rsx! {},
        Question::EmptyTrash => erase(browse, erase_words(None), "Empty Trash"),
        Question::DeleteForGood(items) => {
            let first = items
                .first()
                .map(|item| item.original.name.display().into_owned())
                .unwrap_or_default();
            let title = erase_words(Some((items.len(), first)));
            erase(browse, title, "Delete Immediately")
        }
        Question::Conflict(conflict) => rsx! {
            ConflictAlert { browse, conflict }
        },
        // A question files-model adds later is not asked until this names it.
        _ => rsx! {},
    }
}

/// "Are you sure you want to permanently erase ...?" with the destructive button and Cancel.
fn erase(browse: Browse, title: String, action: &str) -> Element {
    let confirm = EventHandler::new(move |()| browse.send(Input::Confirm));
    let cancel = EventHandler::new(move |()| browse.send(Input::Dismiss));
    rsx! {
        Alert {
            title,
            message: Some(TextLine::from(CANNOT_UNDO)),
            style: AlertStyle::Warning,
            buttons: vec![
                AlertButton::new(action, AlertRole::Destructive, confirm),
                AlertButton::new("Cancel", AlertRole::Cancel, cancel),
            ],
        }
    }
}

/// A taken name: Keep Both, Replace, Skip (when more items come after) or Stop, and "Apply to
/// All" for the rest.
#[component]
fn ConflictAlert(browse: Browse, conflict: Conflict) -> Element {
    let mut all = use_signal(|| Check::Off);
    let (title, message) = conflict_words(&conflict);
    let answer = move |answer: Answer| {
        let resolution = match all() {
            Check::On => Resolution::for_rest(answer),
            Check::Off | Check::Mixed => Resolution::once(answer),
        };
        browse.send(Input::Answer(resolution));
    };
    let button = move |label: &str, role: AlertRole, answer_with: Answer| {
        AlertButton::new(
            label,
            role,
            EventHandler::new(move |()| answer(answer_with)),
        )
    };
    let mut buttons = vec![
        button("Keep Both", AlertRole::Normal, Answer::KeepBoth),
        button("Replace", AlertRole::Normal, Answer::Replace),
    ];
    let suppression = if conflict.rest > 0 {
        buttons.push(button("Skip", AlertRole::Normal, Answer::Skip));
        Some(Suppression {
            label: "Apply to All".to_owned(),
            value: all(),
            onchange: EventHandler::new(move |now: Check| all.set(now)),
        })
    } else {
        None
    };
    buttons.push(button("Stop", AlertRole::Cancel, Answer::Stop));
    rsx! {
        Alert {
            title,
            message: Some(TextLine::from(message)),
            buttons,
            suppression,
        }
    }
}
