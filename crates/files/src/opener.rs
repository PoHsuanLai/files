//! Opening files by asking the desktop: `xdg-open`, detached.
//!
//! Provisional: a later change hands files to the desktop crate, which launches with an
//! activation token and the person's chosen app.

use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use files_ui::Opener;

/// Runs `xdg-open <path>` for each file, in a process group of its own so closing the window
/// does not close the app.
#[derive(Debug, Clone, Copy)]
pub struct XdgOpen;

impl Opener for XdgOpen {
    fn open(&self, targets: Vec<PathBuf>) {
        for path in targets {
            let started = Command::new("xdg-open")
                .arg(&path)
                .stdin(Stdio::null())
                .stdout(Stdio::null())
                .stderr(Stdio::null())
                .process_group(0)
                .spawn();
            // Reaped on a thread of its own so a finished opener does not linger as a zombie.
            if let Ok(mut child) = started {
                let _ = std::thread::Builder::new()
                    .name("files-xdg-open".to_owned())
                    .spawn(move || {
                        let _ = child.wait();
                    });
            }
        }
    }
}
