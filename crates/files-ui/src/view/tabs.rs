//! The tab bar: one segment per tab, titled with its folder's name, shown only while there are
//! two or more tabs (as Finder does).

use dioxus::prelude::*;
use ds::components::chrome::column::model::{ColumnAlign, ColumnGap};
use ds::components::chrome::column::view::Column;
use ds::components::controls::choice::Choice;
use ds::components::controls::segmented::Tracking;
use ds::prelude::SegmentedControl;
use ds::style::tokens::control_size::ControlSize;
use files_model::{Input, TabId};

use crate::places::title_words;
use crate::state::Browse;

/// The tab bar, or nothing while the window has one tab.
#[component]
pub(super) fn TabStrip(browse: Browse) -> Element {
    let (choices, front) = {
        let tabs = browse.tabs.read();
        if tabs.len() < 2 {
            return rsx! {};
        }
        let choices: Vec<Choice<TabId>> = tabs
            .iter()
            .map(|tab| Choice::new(tab.id(), title_words(tab.browser().location())))
            .collect();
        (choices, tabs.front_id())
    };
    rsx! {
        Column { gap: ColumnGap::None, align: ColumnAlign::Center,
            SegmentedControl::<TabId> {
                label: "Tabs".to_owned(),
                choices,
                tracking: Tracking::SelectOne(front),
                size: ControlSize::Small,
                onchange: move |id: TabId| browse.send(Input::ShowTab(id)),
            }
        }
    }
}
