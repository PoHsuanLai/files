//! How a folder is shown.

/// The way a window lays out a folder. Grows by one variant per view that is built.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum ViewMode {
    /// Rows with columns (Finder's list view).
    #[default]
    List,
    /// A grid of icons and thumbnails.
    Icons,
}

/// What a folder is sorted by.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub enum SortKey {
    /// The name, as a person reads numbers in it (`file2` before `file10`).
    #[default]
    Name,
    /// When it last changed.
    Modified,
    /// Its size; folders count as no size.
    Size,
    /// Its kind (folders, then by type).
    Kind,
}

/// Which way a sort runs.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum SortOrder {
    /// Smallest, earliest or A first.
    #[default]
    Ascending,
    /// Largest, latest or Z first.
    Descending,
}

/// A sort: by what, which way.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Sort {
    /// By what.
    pub key: SortKey,
    /// Which way.
    pub order: SortOrder,
}

/// Whether hidden entries are shown.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum HiddenFiles {
    /// Entries whose name starts with a dot are left out.
    #[default]
    Left,
    /// Every entry is shown.
    Shown,
}

/// How one folder is shown; remembered per folder.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
#[non_exhaustive]
pub struct FolderPrefs {
    /// The view.
    pub view: ViewMode,
    /// The sort.
    pub sort: Sort,
}

impl ViewMode {
    /// The word a stored preference writes for this view.
    pub fn slug(self) -> &'static str {
        match self {
            ViewMode::List => "list",
            ViewMode::Icons => "icons",
        }
    }

    /// The view a stored word names; `None` for a word this version does not know.
    pub fn from_slug(slug: &str) -> Option<ViewMode> {
        [ViewMode::List, ViewMode::Icons]
            .into_iter()
            .find(|view| view.slug() == slug)
    }
}

impl SortKey {
    /// The word a stored preference writes for this key.
    pub fn slug(self) -> &'static str {
        match self {
            SortKey::Name => "name",
            SortKey::Modified => "modified",
            SortKey::Size => "size",
            SortKey::Kind => "kind",
        }
    }

    /// The key a stored word names; `None` for a word this version does not know.
    pub fn from_slug(slug: &str) -> Option<SortKey> {
        [
            SortKey::Name,
            SortKey::Modified,
            SortKey::Size,
            SortKey::Kind,
        ]
        .into_iter()
        .find(|key| key.slug() == slug)
    }
}

impl SortOrder {
    /// The word a stored preference writes for this order.
    pub fn slug(self) -> &'static str {
        match self {
            SortOrder::Ascending => "ascending",
            SortOrder::Descending => "descending",
        }
    }

    /// The order a stored word names; `None` for a word this version does not know.
    pub fn from_slug(slug: &str) -> Option<SortOrder> {
        [SortOrder::Ascending, SortOrder::Descending]
            .into_iter()
            .find(|order| order.slug() == slug)
    }
}
