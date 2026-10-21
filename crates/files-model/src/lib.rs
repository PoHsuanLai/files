//! The pure state machines of files.
//!
//! [`Browser`] is one window's view of one location: where it is, where it has been, what the
//! folder holds, what is selected, what is being renamed. It is stepped with
//! [`Browser::step`]`(self, input, now) -> (Browser, Vec<Effect>)`: no disk, no clock, no runtime.
//! The caller carries out the [`Effect`]s (list this folder, watch it, rename that file) and feeds
//! the answers back as [`Input`]s.
//!
//! ```
//! use files_core::{Entry, EntryKind, FileName, FolderPath, Location};
//! use files_model::{Browser, Effect, Input, Now};
//! use std::time::Duration;
//!
//! let home = Location::Folder(FolderPath::new("/home/p")?);
//! let (browser, effects) = Browser::new(home.clone());
//! assert_eq!(effects[0], Effect::List(home.clone()));
//!
//! let entry = Entry::new(FileName::new("Notes")?, EntryKind::Folder);
//! let answer = Input::Listed(home, Ok(vec![entry]));
//! let (browser, _) = browser.step(answer, Now(Duration::ZERO));
//! assert_eq!(browser.rows().len(), 1);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod arrange;
mod browser;
mod effect;
mod history;
mod input;
mod listing;
mod nav;
mod open;
mod rename;
mod renaming;
mod reveal;
mod selection;
mod sort;
mod step;
mod typeselect;

pub use arrange::arrange;
pub use browser::Browser;
pub use effect::{Effect, OpenTarget};
pub use history::{Availability, History};
pub use input::{Change, Direction, Input, Modifier, Now};
pub use listing::{ListFailure, Listing};
pub use open::{Opens, opens};
pub use rename::{Rename, RenameProblem};
pub use selection::Selection;
pub use sort::compare;
pub use typeselect::{RESET, TypeSelect};
