//! Copy, move, trash and rename as work that can be asked about, watched, cancelled and undone.
//!
//! An [`OpPlan`](files_core::OpPlan) says what to change. The [`Executor`] does it, synchronously,
//! reporting to a [`Host`]: how far it has come, what to do when a name is taken (it asks for an
//! [`Answer`](files_core::Answer): keep both, replace, skip or stop), and whether to go on
//! ([`CancelToken`]). What it did comes back as a [`Ran`], and a [`Journal`] remembers how to take
//! each piece of work back: a copy is undone by trashing the copies, a move by moving back, a new
//! folder by trashing it if it is still empty, a rename by renaming back, a move to the Trash by
//! putting back. Nothing here follows a symbolic link: a link is copied as a link.
//!
//! A [`Worker`] runs a queue of plans on a thread of its own and reports [`Event`]s to a [`Sink`];
//! it is plain `std` threads and channels, so the caller owns the runtime.
//!
//! ```
//! use std::sync::Arc;
//! use files_core::{FileName, FolderPath, ItemRef, OpPlan};
//! use files_ops::{Executor, Finish, Flow, Host, Journal};
//! use files_trash::FakeTrash;
//!
//! /// A host that never has to be asked anything.
//! struct Quiet;
//! impl Host for Quiet {
//!     fn progress(&mut self, _: files_core::Progress) {}
//!     fn ask(&mut self, _: &files_core::Conflict) -> files_core::Resolution {
//!         files_core::Resolution::once(files_core::Answer::Stop)
//!     }
//!     fn flow(&self) -> Flow { Flow::Go }
//! }
//!
//! let scratch = tempfile::tempdir()?;
//! let exec = Executor::new(Arc::new(FakeTrash::new(scratch.path().join("bin"))));
//! let mut journal = Journal::new();
//!
//! let parent = FolderPath::new(scratch.path())?;
//! let plan = OpPlan::NewFolder { parent, name: FileName::new("untitled folder")? };
//! let ran = exec.run(&plan, &mut Quiet);
//! assert_eq!(ran.finish, Finish::Done);
//! assert!(scratch.path().join("untitled folder").is_dir());
//!
//! journal.record(&ran);
//! journal.undo(&exec, &mut Quiet);
//! assert!(!scratch.path().join("untitled folder").exists());
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod error;
mod exec;
mod host;
mod journal;
mod ran;
mod worker;

pub use exec::Executor;
pub use host::{CancelToken, Flow, Host};
pub use journal::Journal;
pub use ran::{Finish, Ran};
pub use worker::{Event, Finished, Job, Sink, Worker};
