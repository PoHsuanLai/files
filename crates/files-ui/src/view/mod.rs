//! The window: `Ds` over a `SplitView` of the sidebar and the folder, drawn only with quire's
//! components. The views read the browser and map it; nothing here decides anything.

mod busy;
mod chooser;
mod cloud;
mod file_icons;
mod goto;
mod icon;
mod icons;
mod info;
mod menu;
mod pane;
mod preview;
mod questions;
mod quick_look;
mod rename;
mod row;
mod sidebar;
mod table;
mod tabs;
mod toolbar;
mod wanted;
mod window;

use std::sync::Arc;

use dioxus::prelude::*;
use ds::base::spawner::Spawner;
use ds::prelude::{AppStyle, Ds, Material, RootExtent};
use ds_blitz::TokioSpawner;
use ds_settings::use_environment;

use crate::launch::Launch;

use self::file_icons::use_file_icons;
use self::window::Window;

/// files' own CSS: layout only, quire's tokens only.
pub const STYLE: &str = include_str!("style.css");

/// The window's root component: the look from the person's settings and the desktop, then the
/// window itself under quire's `Ds`.
#[component]
pub fn App() -> Element {
    let launch = use_hook(consume_context::<Launch>);
    let spawner = use_hook(|| Arc::new(TokioSpawner::current()) as Arc<dyn Spawner>);
    // The person's look and the desktop's preferences, live: a change re-themes the window.
    let env = use_environment(
        launch.appearance.store.clone(),
        launch.appearance.system.clone(),
        spawner,
    );
    let now = env();
    let icons = use_file_icons(&launch, now.settings.appearance.file_icons.clone());
    use_context_provider(|| icons);
    rsx! {
        Ds {
            appearance: now.settings.appearance.appearance(),
            system: now.system,
            tint_alpha: Some(now.tint_alpha()),
            stack: Some(now.material_stack()),
            typeface: Some(now.settings.appearance.typeface()),
            material: Material::Window,
            extent: RootExtent::Window,
            AppStyle { css: STYLE }
            Window {}
        }
    }
}
