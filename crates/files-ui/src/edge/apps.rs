//! The apps that can open some files, as the window lists them in Open With.

use std::path::PathBuf;

use files_model::AppPick;

/// One app, as the window shows it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppChoiceView {
    /// What choosing it hands back to the opener.
    pub pick: AppPick,
    /// The name its desktop entry gives it.
    pub name: String,
    /// Whether it is the app the person's settings name for the files.
    pub is_default: bool,
    /// The image file of its icon, when it has one; the window draws it as it is. An app with
    /// none shows no image in the menu and the plain app glyph in the sheet.
    pub icon: Option<PathBuf>,
}

/// Which apps are installed and which open what. The binary gives the window one that reads the
/// desktop's entries and settings; this crate names no such thing.
///
/// The question is asked of paths, not of types: a listing carries no type, and the one that
/// answers knows how to find it.
pub trait Apps: Send + Sync {
    /// The apps that open every one of `targets`; the default, if there is one, comes first.
    fn apps_for(&self, targets: &[PathBuf]) -> Vec<AppChoiceView>;

    /// Every installed app.
    fn all_apps(&self) -> Vec<AppChoiceView>;
}

/// No apps at all: the choice is only ever "Other…", which lists nothing.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoApps;

impl Apps for NoApps {
    fn apps_for(&self, _targets: &[PathBuf]) -> Vec<AppChoiceView> {
        Vec::new()
    }

    fn all_apps(&self) -> Vec<AppChoiceView> {
        Vec::new()
    }
}
