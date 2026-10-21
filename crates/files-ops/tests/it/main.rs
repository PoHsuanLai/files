//! files-ops's one test binary: every file below is a module of it.
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may unwrap (CONVENTIONS)

mod changes;
mod conflicts;
mod support;
mod undo;
mod worker;
