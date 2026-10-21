//! The pure state machines of files.
//!
//! [`Browser`] is one window's view of one location: where it is, where it has been, what the
//! folder holds, what is selected, what is being renamed. It is stepped with
//! [`Browser::step`]`(self, input, now) -> (Browser, Vec<Effect>)`: no disk, no clock, no runtime.
//! The caller carries out the [`Effect`]s (list this folder, watch it, rename that file) and feeds
//! the answers back as [`Input`]s. Changing files is the same: `Input::MoveToTrash` asks for
//! `Effect::Run(OpPlan)`, and the workers' reports come back as `Input::Op`.
//!
//! A window holds several browsers as [`Tabs`]: it steps an input for the front tab or opens, closes
//! and switches tabs, and tags each effect with the tab that asked ([`Tabbed`]) so the answer finds its
//! way back to that tab, in front or not ([`Tabs::heard`]).
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

mod app;
mod arrange;
mod asked;
mod browser;
mod changes;
mod effect;
mod error;
mod goto;
mod history;
mod input;
mod listing;
mod look;
mod nav;
mod open;
mod ops;
mod pathname;
mod rename;
mod renaming;
mod reveal;
mod selection;
mod sort;
mod step;
mod tabs;
mod tagging;
mod typeselect;
mod volumes;

pub use app::{AppPick, Chooser};
pub use arrange::arrange;
pub use asked::Bring;
pub use browser::Browser;
pub use effect::{Effect, Mark, OpenTarget};
pub use error::{GoToProblem, ListFailure, RenameProblem};
pub use goto::{GoTo, expand};
pub use history::{Availability, History};
pub use input::{Change, Direction, Input, Modifier, Now};
pub use listing::Listing;
pub use look::{InfoView, Looking};
pub use open::{Opens, opens};
pub use ops::{DropHow, End, OpEvent, Outcome, Question, Work};
pub use rename::Rename;
pub use selection::Selection;
pub use sort::compare;
pub use tabs::{Tab, TabId, Tabbed, Tabs};
pub use tagging::Held;
pub use typeselect::{RESET, TypeSelect};
