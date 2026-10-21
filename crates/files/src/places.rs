//! The person's user folders, found once from the XDG settings (`user-dirs.dirs`).

use files_core::FolderPath;
use files_ui::{PlaceKind, Places};

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
