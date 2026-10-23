//! Opening files with apps: the desktop crate's launcher behind the window's [`Opener`] seam.

use std::path::PathBuf;
use std::sync::Arc;

use ds_helpers::DesktopId;
use files_desktop::{AppChoice, Launcher};
use files_ui::{OpenAs, OpenFailure, Opener};

/// Hands files to the launcher, each to its default app or all to the one the person picked.
///
/// No activation token goes along yet: the window has no way to ask the compositor for one, so
/// the app that opens raises itself as its own policy allows. When quire offers one, it is
/// passed here.
pub struct DesktopOpener {
    launcher: Arc<dyn Launcher>,
}

impl std::fmt::Debug for DesktopOpener {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("DesktopOpener")
    }
}

impl DesktopOpener {
    /// An opener over `launcher`.
    pub fn new(launcher: Arc<dyn Launcher>) -> DesktopOpener {
        DesktopOpener { launcher }
    }
}

/// The launcher's choice for the window's: a picked app is looked up by its desktop id.
fn choice_of(with: OpenAs) -> Result<AppChoice, OpenFailure> {
    match with {
        OpenAs::Default => Ok(AppChoice::Default),
        OpenAs::App(pick) => DesktopId::new(pick.as_str())
            .map(AppChoice::App)
            .map_err(|_| OpenFailure("That app can\u{2019}t be found.".to_owned())),
    }
}

impl Opener for DesktopOpener {
    fn open(&self, targets: Vec<PathBuf>, with: OpenAs) -> Result<(), OpenFailure> {
        self.launcher
            .open(&targets, choice_of(with)?, None)
            .map_err(|error| OpenFailure(error.to_string()))
    }
}

#[cfg(test)]
mod tests {
    use files_model::AppPick;

    use super::*;

    #[test]
    fn a_pick_becomes_the_app_of_that_desktop_id() {
        let got = choice_of(OpenAs::App(AppPick::new("viewer.desktop"))).unwrap();
        assert_eq!(
            got,
            AppChoice::App(DesktopId::new("viewer.desktop").unwrap())
        );
        assert_eq!(choice_of(OpenAs::Default).unwrap(), AppChoice::Default);
    }
}
