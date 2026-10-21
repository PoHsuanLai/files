//! The name field of a rename in place.

use dioxus::prelude::*;
use ds::components::fields::text_field_model::Invalid;
use ds::motion::detail::stamp::EventStamp;
use ds::prelude::{FieldBezel, FieldFocus, TextField, TextLine, Validity};
use files_model::{Input, RenameProblem};

use crate::state::Browse;

/// quire's text field in the name cell, in its plain bezel so the name is edited where it reads.
/// Return commits, Escape puts the name back, and leaving the field commits. A name the model
/// refuses stays here with its reason under it.
#[component]
pub(super) fn RenameCell(
    browse: Browse,
    draft: String,
    problem: Option<RenameProblem>,
) -> Element {
    let validity = match problem {
        Some(problem) => Validity::Invalid(Invalid {
            message: TextLine::from(problem.to_string()),
            stamp: EventStamp(0),
        }),
        None => Validity::Valid,
    };
    rsx! {
        TextField {
            label: "Name".to_owned(),
            value: draft,
            bezel: FieldBezel::Plain,
            focus: FieldFocus::OnMount,
            validity,
            oninput: move |text: String| browse.send(Input::EditDraft(text)),
            onsubmit: move |_: String| browse.send(Input::CommitRename),
            onblur: move |()| browse.send(Input::CommitRename),
            onkey: move |event: KeyboardEvent| {
                if event.key() == Key::Escape {
                    browse.send(Input::CancelRename);
                }
            },
        }
    }
}
