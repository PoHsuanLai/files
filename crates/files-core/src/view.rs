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
