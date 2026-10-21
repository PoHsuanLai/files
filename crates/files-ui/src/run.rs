//! Opening the window.

use ds_blitz::{AppConfig, AppId, LaunchError, WindowSize};

use crate::launch::Launch;
use crate::view::App;

/// The desktop application id: the Wayland `app_id`, which the desktop matches to
/// `org.quire.Files.desktop` for the window's name and icon.
pub const APP_ID: &str = "org.quire.Files";

/// Opens the window on `launch` and runs until it closes.
pub fn run(launch: Launch) -> Result<(), LaunchError> {
    // The least the layout fits: the sidebar at its least, and room for the table beside it.
    let size = WindowSize::new(960, 600).with_least(640, 360);
    let runtime = launch.runtime.clone();
    let config = AppConfig::new("Files", size)
        .with_app_id(AppId(APP_ID.to_owned()))
        .with_context(launch);
    let config = match runtime {
        Some(handle) => config.with_runtime(handle),
        None => config,
    };
    ds_blitz::launch(App, config)
}
