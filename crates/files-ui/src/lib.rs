//! The window of files: a Finder-like folder view drawn only with quire's components.
//!
//! The parts, top to bottom:
//!
//! - [`Launch`] is everything the window is given at start: the folder to open, the person's
//!   favourite places, where the look comes from, and the seams to the outside ([`Seams`]).
//!   The binary reads the environment once and builds it; the window reads nothing itself.
//! - [`Requests`] is the seam for other programs' asks (show this folder, reveal these files): the
//!   binary fills it from the bus, the window turns each into a model input and raises itself.
//! - [`edge`] carries out what `files-model` asks for (list this folder, watch it, change files,
//!   open these files) against the traits of `files-fs`, `files-trash` and `files-ops` and the seams
//!   of this crate, and feeds the answers back as inputs.
//! - [`words`] are the pure functions that say sizes, kinds and dates the way a person reads them.
//! - [`places`], [`rows`] and [`menu`] are the pure mappings from the model to what quire draws:
//!   the sidebar's sections, the table's selection and sort, the context menu's commands.
//! - [`run`] opens the window; its `view` is `Ds`, `SplitView`, `Sidebar`, `Toolbar` and
//!   `VirtualTable`, and nothing is drawn by hand.
//!
//! ```
//! use files_ui::words::size_words;
//! use files_core::ByteSize;
//!
//! assert_eq!(size_words(ByteSize(1_500_000)), "1.5 MB");
//! ```

pub mod edge;
mod error;
mod keys;
mod launch;
pub mod menu;
pub mod places;
mod requests;
pub mod rows;
mod run;
mod state;
mod view;
pub mod words;

pub use edge::{Opener, Seams, Zone};
pub use error::OpenFailure;
pub use launch::{AppearanceSource, Launch};
pub use places::{Heading, PlaceKey, PlaceKind, Places};
pub use requests::{RaiseToken, Request, RequestSender, Requests, channel};
pub use run::{APP_ID, run};
pub use view::{App, STYLE};
