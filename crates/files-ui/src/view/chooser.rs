//! Open With, Other: a quire `Sheet` listing every installed app. Choosing one opens the
//! selection with it.

use dioxus::prelude::*;
use ds::base::vocab::RowState;
use ds::components::chrome::column::model::ColumnGap;
use ds::components::controls::button_model::{Answers, ButtonFocus};
use ds::prelude::{
    Button, Column, ColumnAlign, Form, List, ListItem, Row, RowLeading, Selection, Sheet,
};
use files_model::{AppPick, Chooser, Input};

use crate::menu::{AppLine, app_names};
use crate::state::Browse;

/// The sheet of apps, while it is up.
#[component]
pub(super) fn AppChooser(browse: Browse) -> Element {
    match browse.state.read().chooser() {
        Chooser::Open => rsx! {
            ChooserSheet { browse }
        },
        Chooser::Closed => rsx! {},
    }
}

/// One app's row.
#[component]
fn AppRow(line: AppLine, selection: Selection, onpick: EventHandler<AppPick>) -> Element {
    let pick = line.pick.clone();
    rsx! {
        Row {
            leading: RowLeading::None,
            title: line.title,
            state: RowState {
                selection,
                ..RowState::default()
            },
            onclick: move |_| onpick.call(pick.clone()),
        }
    }
}

/// The sheet itself: the list of apps and Cancel.
#[component]
fn ChooserSheet(browse: Browse) -> Element {
    let mut cursor = use_signal(|| None::<AppPick>);
    let lines = app_names(browse.all_apps());
    let cancel = EventHandler::new(move |()| browse.send(Input::CloseChooseApp));
    let pick = EventHandler::new(move |app: AppPick| browse.send(Input::OpenWith(app)));
    let select = EventHandler::new(move |app: AppPick| cursor.set(Some(app)));
    let here = cursor();
    let items: Vec<ListItem<AppPick>> = lines
        .into_iter()
        .map(|line| {
            let selection = if here.as_ref() == Some(&line.pick) {
                Selection::Selected
            } else {
                Selection::Unselected
            };
            let (key, label) = (line.pick.clone(), line.title.clone());
            ListItem::row(
                key,
                label,
                rsx! { AppRow { line, selection, onpick: pick } },
            )
        })
        .collect();
    rsx! {
        Sheet {
            label: "Choose Application".to_owned(),
            onclose: cancel,
            Form {
                List::<AppPick> {
                    label: "Applications".to_owned(),
                    items,
                    cursor: here,
                    onselect: Some(select),
                    onpick: Some(pick),
                }
                Column { align: ColumnAlign::End, gap: ColumnGap::default(),
                    // The sheet takes the keyboard as it opens (the menu that opened it is gone),
                    // so Escape reaches it; quire's List has no focus on mount yet.
                    Button {
                        label: "Cancel",
                        answers: Answers::Escape,
                        focus: ButtonFocus::OnMount,
                        onclick: move |_| cancel.call(()),
                    }
                }
            }
        }
    }
}
