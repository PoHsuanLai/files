//! The words of the sidebar.

use crate::places::{Heading, PlaceKind};

/// The row's name for `kind`.
pub fn place_label(kind: PlaceKind) -> &'static str {
    match kind {
        PlaceKind::Home => "Home",
        PlaceKind::Desktop => "Desktop",
        PlaceKind::Documents => "Documents",
        PlaceKind::Downloads => "Downloads",
        PlaceKind::Pictures => "Pictures",
        PlaceKind::Music => "Music",
        PlaceKind::Videos => "Videos",
        PlaceKind::Computer => "Computer",
        PlaceKind::Trash => "Trash",
    }
}

/// The heading's words.
pub fn heading_label(heading: Heading) -> &'static str {
    match heading {
        Heading::Favorites => "Favorites",
        Heading::Locations => "Locations",
        Heading::Cloud => "Cloud",
    }
}
