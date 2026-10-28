//! Copying and moving items to a folder, and settling the names that are taken.

use std::fs;
use std::os::unix::fs::MetadataExt;

use files_core::{
    Answer, ByteSize, Conflict, ConflictKind, FileName, FolderPath, ItemRef, OpFailure, Progress,
    Resolution, Split, Transfer, keep_both,
};

use super::tree::{bytes_under, remove_tree};
use super::{Run, Step};
use crate::error;
use crate::host::Flow;
use crate::journal::Inverse;

/// What was asked of the items.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum How {
    Copy,
    Move,
    /// Moves when the folder is on the items' disk, copies otherwise.
    Drag,
}

/// What is actually done to one item.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Mode {
    Copy,
    Move,
}

/// Where an item goes, once the name is settled.
enum Place {
    At(FileName),
    Skip,
    Stop,
}

impl Run<'_> {
    pub(super) fn transfer(&mut self, how: How, transfer: &Transfer) -> Step {
        let destination = &transfer.destination;
        let folder = fs::metadata(destination.as_path()).map_err(error::io)?;
        if !folder.is_dir() {
            return Err(OpFailure::NotFound);
        }
        let bytes: u64 = transfer
            .sources
            .iter()
            .map(|source| bytes_under(&source.path()))
            .sum();
        self.progress = Progress::start(transfer.sources.len(), ByteSize(bytes));
        self.emit();
        self.touch_folder(destination);
        let count = transfer.sources.len();
        for (index, source) in transfer.sources.iter().enumerate() {
            if self.host.flow() == Flow::Stop {
                return Ok(Flow::Stop);
            }
            let rest = count - index - 1;
            if self.one(how, source, destination, folder.dev(), rest)? == Flow::Stop {
                return Ok(Flow::Stop);
            }
            self.progress = self.progress.finished_one();
            self.emit();
        }
        Ok(Flow::Go)
    }

    fn one(
        &mut self,
        how: How,
        source: &ItemRef,
        destination: &FolderPath,
        disk: u64,
        rest: usize,
    ) -> Step {
        let from = source.path();
        let meta = fs::symlink_metadata(&from).map_err(error::io)?;
        let mode = match how {
            How::Copy => Mode::Copy,
            How::Move => Mode::Move,
            How::Drag if meta.dev() == disk => Mode::Move,
            How::Drag => Mode::Copy,
        };
        if meta.is_dir() && destination.as_path().starts_with(&from) {
            return Err(OpFailure::IntoItself);
        }
        let here = source.folder == *destination;
        if here && mode == Mode::Move {
            return Ok(Flow::Go);
        }
        let split = if meta.is_dir() {
            Split::Whole
        } else {
            Split::Extension
        };
        let name = match self.settle(source, destination, mode, split, here, rest)? {
            Place::At(name) => name,
            Place::Skip => return Ok(Flow::Go),
            Place::Stop => return Ok(Flow::Stop),
        };
        let target = ItemRef::new(destination.clone(), name);
        match mode {
            Mode::Copy => self.copy_one(&from, &target),
            Mode::Move => self.move_one(source, &target),
        }
    }

    fn copy_one(&mut self, from: &std::path::Path, target: &ItemRef) -> Step {
        match self.copy_tree(from, &target.path()) {
            Ok(Flow::Go) => {
                self.steps.push(Inverse::Trash(vec![target.clone()]));
                Ok(Flow::Go)
            }
            Ok(Flow::Stop) => {
                let _ = remove_tree(&target.path());
                Ok(Flow::Stop)
            }
            Err(failure) => {
                let _ = remove_tree(&target.path());
                Err(failure)
            }
        }
    }

    fn move_one(&mut self, source: &ItemRef, target: &ItemRef) -> Step {
        if self.move_path(&source.path(), &target.path())? == Flow::Stop {
            return Ok(Flow::Stop);
        }
        self.steps.push(Inverse::MoveBack {
            now: target.clone(),
            back: source.clone(),
        });
        self.touch_folder(&source.folder);
        Ok(Flow::Go)
    }

    /// The name `source` takes in `destination`: its own when free; when taken, what the person
    /// answers (or, for a copy into its own folder, the next free `name 2`, with no question).
    fn settle(
        &mut self,
        source: &ItemRef,
        destination: &FolderPath,
        mode: Mode,
        split: Split,
        here: bool,
        rest: usize,
    ) -> Result<Place, OpFailure> {
        let name = &source.name;
        let taken = |candidate: &FileName| {
            fs::symlink_metadata(destination.join(candidate).as_path()).is_ok()
        };
        let another = || {
            keep_both(name, split, taken)
                .map(Place::At)
                .map_err(|_| OpFailure::NameTooLong)
        };
        if !taken(name) {
            return Ok(Place::At(name.clone()));
        }
        if here {
            return another();
        }
        let kind = match mode {
            Mode::Copy => ConflictKind::Copy,
            Mode::Move => ConflictKind::Move,
        };
        let conflict = Conflict::new(name.clone(), destination.clone(), kind, rest);
        let resolution = match self.sticky {
            Some(resolution) => resolution,
            None => self.host.ask(&conflict),
        };
        if resolution.to_rest {
            self.sticky = Some(Resolution::for_rest(resolution.answer));
        }
        match resolution.answer {
            Answer::KeepBoth => another(),
            Answer::Skip => Ok(Place::Skip),
            Answer::Stop => Ok(Place::Stop),
            Answer::Replace => {
                let existing = ItemRef::new(destination.clone(), name.clone());
                let thrown = self.trash.trash(&[existing]).map_err(error::trash)?;
                self.steps.push(Inverse::PutBack(thrown));
                self.touch_trash();
                Ok(Place::At(name.clone()))
            }
            // An answer files-core adds later is read as Stop until this names it.
            _ => Ok(Place::Stop),
        }
    }
}
