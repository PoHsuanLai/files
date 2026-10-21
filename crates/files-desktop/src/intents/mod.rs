//! Files as an app of docket, the desktop's agent layer: `org.quire.IntentProvider1` served on the
//! app's own bus name, `org.quire.Files`, so the router can ask the window to show a folder, trash
//! or copy or move items, make a folder, ask about emptying the Trash, and say what is selected or
//! which folder is shown. The command line of the app is generated from the same declarations
//! (`dist/intents/org.quire.Files.toml`), through docket; nothing here parses arguments.
//!
//! The window is behind [`Window`]: the binary implements it over its request channel, a test
//! with a fake. Changes are queued on the window's own queue, so they show progress and the
//! window's Undo takes them back; the router's undo goes through [`Ask::Undo`], which the window
//! carries out only if the newest change is the one the token names.

mod args;
mod ask;
mod names;
mod provider;
mod serve;

pub use ask::{Ask, Reply, Undoing, Window};
pub use names::{
    APP_BUS_NAME, COPY, CURRENT_FOLDER, EMPTY_TRASH, MANIFEST, MOVE, NEW_FOLDER, OPEN, SELECTION,
    TRASH,
};
pub use serve::offer;
#[cfg(feature = "testing")]
pub use serve::offer_for_router;
