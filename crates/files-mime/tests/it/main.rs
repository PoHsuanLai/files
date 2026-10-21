//! files-mime's one test binary: every file below is a module of it. All text is inline fixtures;
//! nothing reads the real system.
#![allow(clippy::unwrap_used, clippy::expect_used)] // tests may unwrap (CONVENTIONS)

use files_core::{FileName, MimeType};

mod apps;
mod db;
mod globs;
mod loader;

/// A type from text.
fn mt(text: &str) -> MimeType {
    MimeType::new(text).unwrap()
}

/// A file name from text.
fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}
