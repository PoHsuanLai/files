//! The context menu on a row or on the folder's background, drawn with quire's `Menu`.

use dioxus::prelude::*;
use ds::host::measure::Anchor;
use ds::prelude::{Availability, Menu, MenuItem, MenuPlacement, Point, Px};
use files_model::Availability as Can;

use crate::menu::{Command, Over, groups_for};
use crate::state::Browse;

/// Where a context menu was asked for, and over what.
#[derive(Debug, Clone, Copy, PartialEq)]
pub(super) struct ContextAt {
    /// The pointer, in the window's coordinates.
    pub(super) at: Point,
    /// What it was over.
    pub(super) over: Over,
}

/// The pointer's place in a press, as quire's placement takes it.
pub(super) fn point_of(event: &MouseEvent) -> Point {
    let at = event.client_coordinates();
    Point {
        x: Px(at.x as f32),
        y: Px(at.y as f32),
    }
}

fn availability(can: Can) -> Availability {
    match can {
        Can::Available => Availability::Enabled,
        Can::Unavailable => Availability::Disabled,
    }
}

/// The menu's lines: each group's commands, a rule between groups.
fn items(browse: Browse, over: Over) -> Vec<MenuItem<Command>> {
    let groups = groups_for(&browse.state.read(), over);
    let mut lines = Vec::new();
    for group in groups {
        if !lines.is_empty() {
            lines.push(MenuItem::Separator);
        }
        for (command, can) in group {
            lines
                .push(MenuItem::new(command, command.title()).with_availability(availability(can)));
        }
    }
    lines
}

/// The context menu, while one is asked for.
#[component]
pub(super) fn ContextMenu(browse: Browse, menu: Signal<Option<ContextAt>>) -> Element {
    let mut menu = menu;
    let Some(ContextAt { at, over }) = menu() else {
        return rsx! {};
    };
    rsx! {
        Menu::<Command> {
            placement: MenuPlacement::Context,
            anchor: Anchor::Point(at),
            items: items(browse, over),
            onpick: move |command: Command| browse.send(command.input()),
            onclose: move |()| menu.set(None),
        }
    }
}
