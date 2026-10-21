//! The sidebar's places: which folders are the person's favourites, and how they are grouped.
//!
//! Pure data and mappings. The binary finds the folders (the XDG user directories that exist) and
//! passes them in as [`Places`]; [`groups`] says what the sidebar lists; `view` draws it.

use files_core::{FolderPath, Location};

/// A place the sidebar can list.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKind {
    /// The person's home folder.
    Home,
    /// The Desktop folder.
    Desktop,
    /// The Documents folder.
    Documents,
    /// The Downloads folder.
    Downloads,
    /// The Pictures folder.
    Pictures,
    /// The Music folder.
    Music,
    /// The Videos folder.
    Videos,
    /// The whole filesystem, `/`.
    Computer,
    /// The Trash.
    Trash,
}

impl PlaceKind {
    /// The favourites, in the order the sidebar lists them.
    pub const FAVORITES: [PlaceKind; 7] = [
        PlaceKind::Home,
        PlaceKind::Desktop,
        PlaceKind::Documents,
        PlaceKind::Downloads,
        PlaceKind::Pictures,
        PlaceKind::Music,
        PlaceKind::Videos,
    ];

    /// The row's name.
    pub fn label(self) -> &'static str {
        match self {
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
}

/// A group of rows in the sidebar, named by a heading.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Heading {
    /// The person's favourite folders.
    Favorites,
    /// Where things are kept: the computer's disk.
    Locations,
}

impl Heading {
    /// The heading's words.
    pub fn label(self) -> &'static str {
        match self {
            Heading::Favorites => "Favorites",
            Heading::Locations => "Locations",
        }
    }
}

/// What one sidebar item stands for: a heading, or a place.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum PlaceKey {
    /// A group's heading, which the keys pass over.
    Heading(Heading),
    /// A place the person can go to.
    Place(PlaceKind),
}

/// The folders behind the favourites, as the binary found them. A favourite with no folder (the
/// person has no Music folder) is simply not listed.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Places {
    folders: Vec<(PlaceKind, FolderPath)>,
}

impl Places {
    /// `kind` is `folder`, replacing any folder it had. [`PlaceKind::Computer`] is always `/` and
    /// ignores this.
    pub fn with(mut self, kind: PlaceKind, folder: FolderPath) -> Places {
        self.folders.retain(|(known, _)| *known != kind);
        self.folders.push((kind, folder));
        self
    }

    /// The place `kind` goes to, when it has one.
    pub fn location_of(&self, kind: PlaceKind) -> Option<Location> {
        match kind {
            PlaceKind::Computer => FolderPath::new("/").ok().map(Location::Folder),
            PlaceKind::Trash => Some(Location::Trash),
            PlaceKind::Home
            | PlaceKind::Desktop
            | PlaceKind::Documents
            | PlaceKind::Downloads
            | PlaceKind::Pictures
            | PlaceKind::Music
            | PlaceKind::Videos => self
                .folders
                .iter()
                .find(|(known, _)| *known == kind)
                .map(|(_, folder)| Location::Folder(folder.clone())),
        }
    }

    /// The place `location` is, for the sidebar's cursor: the first listed place whose folder it
    /// is, favourites before the computer. A folder that is none of them has no place.
    pub fn key_of(&self, location: &Location) -> Option<PlaceKey> {
        PlaceKind::FAVORITES
            .into_iter()
            .chain([PlaceKind::Computer, PlaceKind::Trash])
            .find(|kind| self.location_of(*kind).as_ref() == Some(location))
            .map(PlaceKey::Place)
    }
}

/// One group of the sidebar: a heading and the places under it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Group {
    /// The heading.
    pub heading: Heading,
    /// The rows, in order.
    pub rows: Vec<PlaceKind>,
}

/// What the sidebar lists: the favourites that have a folder (none, no group), then the computer
/// and the Trash.
///
/// Recents is left out until the place behind it is built.
pub fn groups(places: &Places) -> Vec<Group> {
    let favorites: Vec<PlaceKind> = PlaceKind::FAVORITES
        .into_iter()
        .filter(|kind| places.location_of(*kind).is_some())
        .collect();
    let mut groups = Vec::new();
    if !favorites.is_empty() {
        groups.push(Group {
            heading: Heading::Favorites,
            rows: favorites,
        });
    }
    groups.push(Group {
        heading: Heading::Locations,
        rows: vec![PlaceKind::Computer, PlaceKind::Trash],
    });
    groups
}

/// The words for the toolbar's title and the window's: the folder's own name (`/` for the
/// filesystem's root), or `Trash`.
pub fn title_words(location: &Location) -> String {
    match location {
        Location::Folder(folder) => folder.as_path().file_name().map_or_else(
            || "/".to_owned(),
            |name| name.to_string_lossy().into_owned(),
        ),
        Location::Trash => "Trash".to_owned(),
        // A place files-core adds later has no name until this gives it one.
        _ => "Files".to_owned(),
    }
}
