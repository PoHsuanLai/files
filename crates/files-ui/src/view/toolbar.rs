//! The folder's toolbar: Back and Forward, the folder's name over the view switcher.

use dioxus::prelude::*;
use ds::components::chrome::column::model::{ColumnAlign, ColumnGap};
use ds::components::chrome::column::view::Column;
use ds::components::chrome::toolbar::model::{Picked, ToolbarItem};
use ds::components::chrome::toolbar::view::Toolbar;
use ds::components::controls::choice::Choice;
use ds::components::controls::segmented::Tracking;
use ds::components::content::label::LabelStyle;
use ds::prelude::{Availability, Icon, Label, SegmentedControl, Shown};
use ds::style::tokens::control_size::ControlSize;
use files_core::ViewMode;
use files_model::Input;

use crate::places::title_words;
use crate::state::Browse;

/// What a press on the toolbar asks for.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Tool {
    Back,
    Forward,
    ShowSidebar,
}

/// The toolbar's availability for the model's.
fn live(can: files_model::Availability) -> Availability {
    match can {
        files_model::Availability::Available => Availability::Enabled,
        files_model::Availability::Unavailable => Availability::Disabled,
    }
}

/// The view switcher's choices. Icons waits for quire's collection view, so it is shown and
/// cannot be picked.
fn view_choices() -> Vec<Choice<ViewMode>> {
    vec![
        Choice::new(ViewMode::List, "List"),
        Choice::new(ViewMode::Icons, "Icons").with_availability(Availability::Disabled),
    ]
}

/// The toolbar over the folder: Back and Forward (and Show Sidebar while it is folded away), and
/// in the middle the folder's name above the List and Icons switcher.
#[component]
pub(super) fn FolderBar(browse: Browse, sidebar: Signal<Shown>) -> Element {
    let (title, back, forward, view) = {
        let state = browse.state.read();
        (
            title_words(state.location()),
            live(state.can_go_back()),
            live(state.can_go_forward()),
            state.prefs().view,
        )
    };
    let mut leading = Vec::new();
    if sidebar() == Shown::Hidden {
        leading.push(ToolbarItem::new(
            Tool::ShowSidebar,
            "Show Sidebar",
            Icon::PanelLeft,
        ));
    }
    leading.push(ToolbarItem::new(Tool::Back, "Back", Icon::ChevronLeft).with(back));
    leading.push(ToolbarItem::new(Tool::Forward, "Forward", Icon::ChevronRight).with(forward));
    let mut sidebar = sidebar;
    rsx! {
        Toolbar::<Tool> {
            leading,
            center: rsx! {
                Column { gap: ColumnGap::None, align: ColumnAlign::Center,
                    Label { text: title, style: LabelStyle::Headline }
                    SegmentedControl::<ViewMode> {
                        label: "View".to_owned(),
                        choices: view_choices(),
                        tracking: Tracking::SelectOne(view),
                        size: ControlSize::Small,
                        onchange: move |mode: ViewMode| browse.send(Input::SetView(mode)),
                    }
                }
            },
            onpick: move |picked: Picked<Tool>| match picked.value {
                Tool::Back => browse.send(Input::Back),
                Tool::Forward => browse.send(Input::Forward),
                Tool::ShowSidebar => sidebar.set(Shown::Visible),
            },
        }
    }
}
