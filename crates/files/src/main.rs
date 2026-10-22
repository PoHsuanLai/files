//! `files`: the desktop's file manager.
//!
//! This is the only place the program reads its arguments and its environment: the folder to
//! open, the person's home and user folders, the XDG folders and the working folder. They are
//! passed down as values; the window reads nothing itself. `main` parses, builds the one runtime,
//! claims the running instance (or hands its request to it) and opens the window. See `args.rs`
//! for the command line.

mod apps;
mod args;
mod bridge;
mod icons;
mod launcher;
mod opener;
mod places;
mod port;
mod session;
mod state;
mod thumbnails;
mod volumes;

use std::process::ExitCode;
use std::sync::Arc;

use ds_settings::ConfigRoot;
use files_core::{FolderPath, Location};
use files_desktop::{Disks, ProcUsers};
use files_ui::Launch;

use crate::apps::InstalledApps;
use crate::args::{Args, Start};
use crate::opener::DesktopOpener;
use crate::session::{Role, join, launch_token};
use crate::volumes::DesktopVolumes;

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(said) => {
            eprintln!("files: {said}");
            eprintln!("usage: files [FOLDER]");
            return ExitCode::from(2);
        }
    };
    let folder = match args.start {
        Start::Trash => Some(Location::Trash),
        Start::Given(path) => folder_at(absolute(path, std::env::current_dir().ok())),
        Start::Home => folder_at(dirs::home_dir()),
    };
    let Some(start) = folder else {
        eprintln!("files: there is no folder to open; give one: files FOLDER");
        return ExitCode::FAILURE;
    };
    let runtime = match tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
    {
        Ok(runtime) => runtime,
        Err(err) => {
            eprintln!("files: {err}");
            return ExitCode::FAILURE;
        }
    };
    let role = runtime.block_on(join(
        args.started,
        start.clone(),
        launch_token(),
        runtime.handle().clone(),
    ));
    let Role::App(running) = role else {
        return ExitCode::SUCCESS;
    };
    let launcher = Arc::new(launcher::launcher(&launcher::xdg_dirs(), running.activate));
    let launch = Launch::new(start, Arc::new(DesktopOpener::new(launcher.clone())))
        .with_apps(Arc::new(InstalledApps::new(launcher)))
        .with_places(places::found())
        .with_prefs(state::prefs_store(&ConfigRoot::Xdg))
        .with_thumbnails(thumbnails::shared())
        .with_requests(running.requests)
        .with_runtime(runtime.handle().clone());
    // Without the system bus the window has no disks to list.
    let launch = match runtime.block_on(Disks::system(Arc::new(ProcUsers::new("/proc")))) {
        Ok(disks) => launch.with_volumes(Arc::new(DesktopVolumes::new(
            disks,
            runtime.handle().clone(),
        ))),
        Err(_) => launch,
    };
    let launch = match dirs::home_dir().and_then(|home| FolderPath::new(home).ok()) {
        Some(home) => launch.with_home(home),
        None => launch,
    };
    match files_ui::run(launch) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("files: {err}");
            ExitCode::FAILURE
        }
    }
}

/// The folder place `path` names, when there is a path and it is absolute.
fn folder_at(path: Option<std::path::PathBuf>) -> Option<Location> {
    FolderPath::new(path?).ok().map(Location::Folder)
}

/// `path` made absolute against the working folder `cwd`; a relative path with no working folder
/// stays relative, and the window then has no folder to open.
fn absolute(
    path: std::path::PathBuf,
    cwd: Option<std::path::PathBuf>,
) -> Option<std::path::PathBuf> {
    if path.is_absolute() {
        Some(path)
    } else {
        cwd.map(|cwd| cwd.join(path))
    }
}
