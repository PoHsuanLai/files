//! The choices a person makes about how a window behaves, as plain data.
//!
//! The program reads them from its settings file and hands them down; nothing here reads a file.
//! Every default is Finder's.

/// How a window behaves where the person has a choice.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Behaviour {
    /// Whether folders come first when sorting by name, in either direction (Finder's default is
    /// mixed in name order).
    pub folders_first: bool,
    /// Whether emptying the Trash is asked about first (Finder's default).
    pub ask_before_emptying_trash: bool,
    /// Whether changing a file's extension is asked about first (Finder's default).
    pub ask_before_changing_extension: bool,
}

impl Default for Behaviour {
    fn default() -> Behaviour {
        Behaviour {
            folders_first: false,
            ask_before_emptying_trash: true,
            ask_before_changing_extension: true,
        }
    }
}
