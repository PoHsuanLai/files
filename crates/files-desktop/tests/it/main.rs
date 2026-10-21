//! files-desktop's one test binary: every file below is a module of it. The bus tests run on a
//! private `dbus-daemon` with a scratch config and skip, saying why, when there is none; nothing
//! touches the person's session bus or home folder.
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may unwrap (CONVENTIONS)

mod bus;
mod exec;
mod file_manager;
mod instance;
mod launch;
mod uri;
