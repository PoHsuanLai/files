//! Thumbnails for the icon view: a picture of each file that has one, in place of its icon.
//!
//! A thumbnail is looked up in the cache the desktop shares (the freedesktop.org thumbnail
//! specification, behind anyview's [`ThumbnailCache`](anyview_platform::ThumbnailCache)); when
//! there is none, anyview's peek makes one and it is kept there for every program. Only the tiles
//! on screen want one: [`Queue`] orders the wishes (the newest first) and drops the answer for a
//! tile that scrolled away; [`Thumbnailer`] does one thumbnail; [`Thumbs`] holds the answers the
//! window has had. The threads that run it are the edge's.
//!
//! ```
//! use files_ui::thumbs::Queue;
//!
//! assert!(Queue::default().is_empty());
//! ```

mod key;
mod make;
mod queue;
mod run;
mod shelf;

pub use key::{ThumbKey, Thumbed, Version};
pub use make::{Anyview, Made, Maker, SIZE};
pub use queue::{Delivery, Queue};
pub use run::Thumbnailer;
pub use shelf::Thumbs;
