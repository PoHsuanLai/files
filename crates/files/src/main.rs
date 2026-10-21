//! `files`: the desktop's file manager.
//!
//! This is the only place the program reads its arguments and its environment: the folder to
//! open, the person's home and user folders, and the working folder. They are passed down as
//! values; the window reads nothing itself. See `args.rs` for the command line.

mod args;
mod opener;
mod places;

use std::process::ExitCode;

use files_core::{FolderPath, Location};
use files_ui::Launch;

use crate::args::{Args, Start};
use crate::opener::XdgOpen;

fn main() -> ExitCode {
    let args = match Args::parse(std::env::args_os().skip(1)) {
        Ok(args) => args,
        Err(said) => {
            eprintln!("files: {said}");
            eprintln!("usage: files [FOLDER]");
            return ExitCode::from(2);
        }
    };
    let places = places::found();
    let start = match args.start {
        Start::Given(path) => absolute(path, std::env::current_dir().ok()),
        Start::Home => dirs::home_dir(),
    };
    let Some(folder) = start.and_then(|path| FolderPath::new(path).ok()) else {
        eprintln!("files: there is no folder to open; give one: files FOLDER");
        return ExitCode::FAILURE;
    };
    let launch =
        Launch::new(Location::Folder(folder), std::sync::Arc::new(XdgOpen)).with_places(places);
    match files_ui::run(launch) {
        Ok(()) => ExitCode::SUCCESS,
        Err(err) => {
            eprintln!("files: {err}");
            ExitCode::FAILURE
        }
    }
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
