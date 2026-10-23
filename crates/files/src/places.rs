//! The person's user folders, found once from the XDG settings (`user-dirs.dirs`).

use std::path::{Path, PathBuf};

use files_core::FolderPath;
use files_ui::settings::NewWindow;
use files_ui::{PlaceKind, Places};

/// The folder a new window opens on: the one chosen, or the home folder when that is not a folder
/// on this computer.
pub fn new_window(choice: NewWindow) -> Option<PathBuf> {
    let home = dirs::home_dir();
    let chosen = match choice {
        NewWindow::Home => home.clone(),
        NewWindow::Desktop => dirs::desktop_dir(),
        NewWindow::Documents => dirs::document_dir(),
        NewWindow::Downloads => dirs::download_dir(),
        NewWindow::Folder(path) => Some(expanded(&path, home.as_deref())),
    };
    chosen.filter(|folder| folder.is_dir()).or(home)
}

/// `path` with a leading `~` standing for `home`.
fn expanded(path: &Path, home: Option<&Path>) -> PathBuf {
    match (path.strip_prefix("~"), home) {
        (Ok(rest), Some(home)) => home.join(rest),
        _ => path.to_path_buf(),
    }
}

/// Home and the user folders that exist, as the sidebar's favorites.
pub fn found() -> Places {
    let folders = [
        (PlaceKind::Home, dirs::home_dir()),
        (PlaceKind::Desktop, dirs::desktop_dir()),
        (PlaceKind::Documents, dirs::document_dir()),
        (PlaceKind::Downloads, dirs::download_dir()),
        (PlaceKind::Pictures, dirs::picture_dir()),
        (PlaceKind::Music, dirs::audio_dir()),
        (PlaceKind::Videos, dirs::video_dir()),
    ];
    folders
        .into_iter()
        .filter_map(|(kind, path)| Some((kind, path.filter(|p| p.is_dir())?)))
        .filter_map(|(kind, path)| Some((kind, FolderPath::new(path).ok()?)))
        .fold(Places::default(), |places, (kind, folder)| {
            places.with(kind, folder)
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_leading_tilde_is_the_home_folder() {
        let home = Some(Path::new("/home/p"));
        let cases = [
            ("a tilde", "~/Work", home, "/home/p/Work"),
            ("only the tilde", "~", home, "/home/p"),
            ("no home", "~/Work", None, "~/Work"),
            ("a full path", "/srv/files", home, "/srv/files"),
            ("a tilde inside a name", "~work/x", home, "~work/x"),
        ];
        for (name, path, home, want) in cases {
            assert_eq!(expanded(Path::new(path), home), Path::new(want), "{name}");
        }
    }
}
