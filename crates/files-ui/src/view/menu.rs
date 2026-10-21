//! The context menu on a row or on the folder's background, drawn with quire's `Menu`.

use dioxus::prelude::*;
use ds::host::measure::Anchor;
use ds::prelude::{
    Availability, Check, IconSize, Menu, MenuImage, MenuItem, MenuPlacement, Point, Px,
};
use files_core::TagColour;
use files_model::Availability as Can;

use crate::menu::{Command, OTHER_APP, Over, Pick, TagMark, app_lines, groups_for, tag_mark};
use crate::state::Browse;

use super::icon::app_icon;

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

/// The Open With submenu: the apps that open the selection, the default first, then a rule and
/// "Other\u{2026}", which lists every installed app.
fn open_with(browse: Browse) -> Vec<MenuItem<Pick>> {
    let mut lines: Vec<MenuItem<Pick>> = app_lines(browse.apps_for_selection())
        .into_iter()
        .map(|line| {
            let item = MenuItem::new(Pick::App(line.pick), line.title);
            match line
                .icon
                .as_deref()
                .and_then(|file| app_icon(file, IconSize::Base))
            {
                Some(icon) => item.with_image(MenuImage::Source(icon)),
                None => item,
            }
        })
        .collect();
    if !lines.is_empty() {
        lines.push(MenuItem::Separator);
    }
    lines.push(MenuItem::new(Pick::OtherApp, OTHER_APP));
    lines
}

/// The Tags submenu: the seven colours, each checked when every selected item has it.
fn tags(browse: Browse) -> Vec<MenuItem<Pick>> {
    let state = browse.state.read();
    TagColour::ALL
        .into_iter()
        .map(|colour| {
            let check = match tag_mark(&state, colour) {
                Some(TagMark::Every) => Check::On,
                Some(TagMark::Part) => Check::Mixed,
                None => Check::Off,
            };
            MenuItem::new(Pick::Tag(colour), colour.label()).with_check(check)
        })
        .collect()
}

/// The menu's lines: each group's commands, a rule between groups, Open With and Tags as
/// submenus.
fn items(browse: Browse, over: Over) -> Vec<MenuItem<Pick>> {
    let groups = groups_for(&browse.state.read(), over);
    let mut lines = Vec::new();
    for group in groups {
        if !lines.is_empty() {
            lines.push(MenuItem::Separator);
        }
        for (command, can) in group {
            let line = if command == Command::OpenWith || command == Command::Tags {
                let children = if command == Command::Tags {
                    tags(browse)
                } else {
                    open_with(browse)
                };
                MenuItem::Submenu {
                    title: command.title().to_owned(),
                    image: None,
                    availability: availability(can),
                    children,
                }
            } else {
                MenuItem::new(Pick::Command(command), command.title())
                    .with_availability(availability(can))
            };
            lines.push(line);
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
        Menu::<Pick> {
            placement: MenuPlacement::Context,
            anchor: Anchor::Point(at),
            items: items(browse, over),
            onpick: move |pick: Pick| {
                if let Some(input) = pick.input() {
                    browse.send(input);
                }
            },
            onclose: move |()| menu.set(None),
        }
    }
}
