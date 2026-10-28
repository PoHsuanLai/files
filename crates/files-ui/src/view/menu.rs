//! The context menu on a row or on the folder's background, drawn with quire's `Menu`.

use dioxus::prelude::*;
use ds::host::measure::Anchor;
use ds::prelude::{Check, IconSize, Menu, MenuImage, MenuItem, MenuPlacement, Point, Px};
use files_model::KeptMark;

use crate::menu::{Command, OTHER_APP, Over, Pick, app_lines, enabled, groups_for};
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

/// The menu's lines: each group's commands, a rule between groups, Open With as a
/// submenu.
/// The check of Keep Downloaded: on when all of the selection is kept, a dash when some is.
fn check_of(kept: KeptMark) -> Check {
    match kept {
        KeptMark::All => Check::On,
        KeptMark::Partly => Check::Mixed,
        KeptMark::Nothing => Check::Off,
    }
}

fn items(browse: Browse, over: Over) -> Vec<MenuItem<Pick>> {
    let groups = groups_for(&browse.state.read(), over);
    let kept = browse.state.read().cloud_menu().map(|menu| menu.kept);
    let mut lines = Vec::new();
    for group in groups {
        if !lines.is_empty() {
            lines.push(MenuItem::Separator);
        }
        for (command, can) in group {
            let line = if command == Command::OpenWith {
                MenuItem::Submenu {
                    title: command.title().to_owned(),
                    image: None,
                    availability: enabled(can),
                    children: open_with(browse),
                }
            } else {
                let line = MenuItem::new(Pick::Command(command), command.title())
                    .with_availability(enabled(can));
                match (command, kept) {
                    (Command::KeepDownloaded, Some(kept)) => line.with_check(check_of(kept)),
                    _ => line,
                }
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
