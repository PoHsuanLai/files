//! The choices a person makes about how a window behaves, as plain data.
//!
//! The program reads them from its settings file and hands them down; nothing here reads a file.
//! Every default is Finder's.

/// Where folders sit among the files when a folder is sorted by name.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FoldersOnTop {
    /// Folders and files are mixed, in name order (Finder's default).
    #[default]
    Mixed,
    /// Folders come first, in either direction.
    First,
}

/// Whether the window asks before it goes on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Asking {
    /// Ask first (Finder's default for both warnings).
    #[default]
    Ask,
    /// Go straight on.
    Skip,
}

/// How a window behaves where the person has a choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Behaviour {
    /// Where folders sit when sorting by name.
    pub folders: FoldersOnTop,
    /// Whether emptying the Trash is asked about first.
    pub emptying_trash: Asking,
    /// Whether changing a file's extension is asked about first.
    pub changing_extension: Asking,
}
