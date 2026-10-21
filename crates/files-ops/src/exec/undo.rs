//! Taking a piece of work back, one step at a time, newest step first.

use std::fs;

use files_core::{ItemRef, OpFailure};

use super::{Next, Run, Step};
use crate::error;
use crate::host::Flow;
use crate::journal::Inverse;

impl Run<'_> {
    /// Applies `steps` last to first. The first step that fails ends it; the ones after it in
    /// the list (done earlier) stay done.
    pub(super) fn undo_steps(&mut self, steps: Vec<Inverse>) -> Step {
        for step in steps.into_iter().rev() {
            if self.host.flow() == Flow::Stop {
                return Ok(Next::Stop);
            }
            self.undo_step(step)?;
        }
        Ok(Next::Go)
    }

    fn undo_step(&mut self, step: Inverse) -> Result<(), OpFailure> {
        match step {
            Inverse::Trash(items) => self.trash_away(&items),
            Inverse::MoveBack { now, back } => {
                if fs::symlink_metadata(back.path()).is_ok() {
                    return Err(OpFailure::Taken);
                }
                if self.move_path(&now.path(), &back.path())? == Next::Stop {
                    return Err(OpFailure::Other);
                }
                self.touch_folder(&now.folder);
                self.touch_folder(&back.folder);
                Ok(())
            }
            Inverse::RemoveEmpty(folder) => {
                let empty = fs::read_dir(folder.path())
                    .map_err(error::io)?
                    .next()
                    .is_none();
                if empty {
                    self.trash_away(&[folder])
                } else {
                    Err(OpFailure::NotEmpty)
                }
            }
            Inverse::RenameBack { now, was } => {
                let back = now.renamed(was);
                if fs::symlink_metadata(back.path()).is_ok() {
                    return Err(OpFailure::Taken);
                }
                fs::rename(now.path(), back.path()).map_err(error::io)?;
                self.touch_folder(&now.folder);
                Ok(())
            }
            Inverse::PutBack(items) => {
                self.trash.put_back(&items).map_err(error::trash)?;
                for item in &items {
                    self.touch_folder(&item.original.folder);
                }
                self.touch_trash();
                Ok(())
            }
        }
    }

    /// Moves `items` to the Trash as part of taking something back.
    fn trash_away(&mut self, items: &[ItemRef]) -> Result<(), OpFailure> {
        self.trash.trash(items).map_err(error::trash)?;
        for item in items {
            self.touch_folder(&item.folder);
        }
        self.touch_trash();
        Ok(())
    }
}
