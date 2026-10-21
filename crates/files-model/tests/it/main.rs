//! files-model's one test binary: every file below is a module of it.
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may unwrap (CONVENTIONS)

mod changes;
mod common;
mod listing;
mod looks;
mod navigation;
mod open;
mod prefs;
mod rename;
mod reveal;
mod selection;
mod sorting;
mod typeselect;
