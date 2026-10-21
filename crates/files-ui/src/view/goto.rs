//! Go to Folder: a quire `Sheet` with a `TextField` holding the path. Return goes, Escape or
//! Cancel puts it away; a folder that is not there is said under the field and the sheet stays.

use dioxus::prelude::*;
use ds::components::chrome::column::model::ColumnGap;
use ds::components::controls::button_model::Answers;
use ds::components::fields::text_field_model::Invalid;
use ds::motion::detail::stamp::EventStamp;
use ds::prelude::{
    Button, Column, ColumnAlign, FieldFocus, Form, Sheet, TextField, TextLine, Validity,
};
use files_model::{GoTo, Input};

use crate::state::Browse;

/// The Go to Folder sheet, while it is up.
#[component]
pub(super) fn GoToSheet(browse: Browse) -> Element {
    let GoTo::Asking { draft, problem, .. } = browse.state.read().go_to().clone() else {
        return rsx! {};
    };
    let validity = match problem {
        Some(problem) => Validity::Invalid(Invalid {
            message: TextLine::from(problem.to_string()),
            stamp: EventStamp(0),
        }),
        None => Validity::Valid,
    };
    let go = EventHandler::new(move |()| browse.send(Input::GoToSubmit));
    let cancel = EventHandler::new(move |()| browse.send(Input::GoToCancel));
    rsx! {
        Sheet {
            label: "Go to Folder".to_owned(),
            onclose: cancel,
            on_return: Some(go),
            Form {
                TextField {
                    label: "Go to the folder".to_owned(),
                    value: draft,
                    focus: FieldFocus::OnMount,
                    validity,
                    oninput: move |text: String| browse.send(Input::GoToTyped(text)),
                }
                Column { align: ColumnAlign::End, gap: ColumnGap::default(),
                    Button {
                        label: "Go",
                        answers: Answers::Return,
                        onclick: move |_| go.call(()),
                    }
                    Button {
                        label: "Cancel",
                        answers: Answers::Escape,
                        onclick: move |_| cancel.call(()),
                    }
                }
            }
        }
    }
}
