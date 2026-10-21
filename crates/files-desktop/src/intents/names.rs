//! The names Files owns on the bus and the actions it declares. The declarations themselves are
//! the file `dist/intents/org.quire.Files.toml`, embedded here so the file the installer ships and
//! the one the window answers for are one file.
//!
//! Action names start with `files`, the app name's last element, which the agent layer requires.

/// The name the agent layer calls (the manifest's app); also the name the running app owns.
pub const APP_BUS_NAME: &str = "org.quire.Files";

/// The shipped actions file.
pub const MANIFEST: &str = include_str!("../../../../dist/intents/org.quire.Files.toml");

/// Show a file or folder.
pub const OPEN: &str = "files.open";
/// Move items to the Trash.
pub const TRASH: &str = "files.trash";
/// Make a folder.
pub const NEW_FOLDER: &str = "files.new_folder";
/// Copy items into a folder.
pub const COPY: &str = "files.copy";
/// Move items into a folder.
pub const MOVE: &str = "files.move";
/// Ask whether to empty the Trash.
pub const EMPTY_TRASH: &str = "files.empty_trash";
/// The selected items.
pub const SELECTION: &str = "files.selection";
/// The folder the window shows.
pub const CURRENT_FOLDER: &str = "files.current_folder";
