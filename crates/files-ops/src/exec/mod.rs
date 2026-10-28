//! The executor: runs a typed plan to the end, reporting to a host.

mod transfer;
mod tree;
mod undo;

use std::fs;
use std::sync::Arc;

use files_core::{
    FileName, Finish, FolderPath, ItemRef, Location, OpFailure, OpPlan, Progress, Resolution,
    Summary, TrashedItem,
};
use files_trash::Trash;

use self::transfer::How;
use crate::error;
use crate::host::{Flow, Host};
use crate::journal::Inverse;
use crate::ran::Ran;

/// The result of one step.
pub(crate) type Step = Result<Flow, OpFailure>;

/// Runs plans on the disk, with a [`Trash`] for the trash and for what a replace puts aside.
///
/// It is synchronous and keeps no state between plans; the journal and the queue are separate
/// (see [`Journal`](crate::Journal) and [`Worker`](crate::Worker)).
#[derive(Clone)]
pub struct Executor {
    trash: Arc<dyn Trash>,
}

impl std::fmt::Debug for Executor {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("Executor")
    }
}

impl Executor {
    /// An executor that uses `trash`.
    pub fn new(trash: Arc<dyn Trash>) -> Executor {
        Executor { trash }
    }

    /// Does `plan`, reporting to `host`. It always comes back with what it did: a failure or a
    /// stop part-way is in the result's `finish`, and what was done before it is in the steps
    /// the journal would take back.
    pub fn run(&self, plan: &OpPlan, host: &mut dyn Host) -> Ran {
        let mut run = Run::new(&*self.trash, host);
        let finish = finish_of(run.plan(plan));
        run.into_ran(plan.summary(), finish)
    }

    /// Takes back `steps`, newest first. Used by the journal.
    pub(crate) fn undo(&self, summary: Summary, steps: Vec<Inverse>, host: &mut dyn Host) -> Ran {
        let mut run = Run::new(&*self.trash, host);
        let finish = finish_of(run.undo_steps(steps));
        run.steps.clear();
        run.into_ran(summary, finish)
    }
}

fn finish_of(step: Step) -> Finish {
    match step {
        Ok(Flow::Go) => Finish::Done,
        Ok(Flow::Stop) => Finish::Stopped,
        Err(failure) => Finish::Failed(failure),
    }
}

/// One plan being done.
pub(crate) struct Run<'a> {
    pub(crate) trash: &'a dyn Trash,
    pub(crate) host: &'a mut dyn Host,
    pub(crate) progress: Progress,
    pub(crate) steps: Vec<Inverse>,
    touched: Vec<Location>,
    /// The answer the person gave for every later conflict.
    pub(crate) sticky: Option<Resolution>,
}

impl<'a> Run<'a> {
    fn new(trash: &'a dyn Trash, host: &'a mut dyn Host) -> Run<'a> {
        Run {
            trash,
            host,
            progress: Progress::default(),
            steps: Vec::new(),
            touched: Vec::new(),
            sticky: None,
        }
    }

    fn into_ran(self, summary: Summary, finish: Finish) -> Ran {
        Ran {
            summary,
            finish,
            touched: self.touched,
            steps: self.steps,
        }
    }

    /// Tells the host how far the work has come.
    pub(crate) fn emit(&mut self) {
        self.host.progress(self.progress);
    }

    /// Notes that `folder` now holds something different.
    pub(crate) fn touch_folder(&mut self, folder: &FolderPath) {
        self.touch(Location::Folder(folder.clone()));
    }

    /// Notes that the Trash now holds something different.
    pub(crate) fn touch_trash(&mut self) {
        self.touch(Location::Trash);
    }

    fn touch(&mut self, place: Location) {
        if !self.touched.contains(&place) {
            self.touched.push(place);
        }
    }

    fn plan(&mut self, plan: &OpPlan) -> Step {
        match plan {
            OpPlan::Copy(transfer) => self.transfer(How::Copy, transfer),
            OpPlan::Move(transfer) => self.transfer(How::Move, transfer),
            OpPlan::Drag(transfer) => self.transfer(How::Drag, transfer),
            OpPlan::NewFolder { parent, name } => self.new_folder(parent, name),
            OpPlan::Rename { item, to } => self.rename(item, to),
            OpPlan::Trash { items } => self.trash_items(items),
            OpPlan::PutBack { items } => self.put_back(items),
            OpPlan::DeleteForGood { items } => self.delete_for_good(items),
            OpPlan::EmptyTrash => self.empty_trash(),
            // A plan files-core adds later cannot be done until this names it.
            _ => Err(OpFailure::Other),
        }
    }

    fn new_folder(&mut self, parent: &FolderPath, name: &FileName) -> Step {
        let made = ItemRef::new(parent.clone(), name.clone());
        fs::create_dir(made.path()).map_err(error::io)?;
        self.steps.push(Inverse::RemoveEmpty(made));
        self.touch_folder(parent);
        Ok(Flow::Go)
    }

    fn rename(&mut self, item: &ItemRef, to: &FileName) -> Step {
        let target = item.renamed(to.clone());
        // `rename(2)` replaces what is at the target; a rename never should.
        if fs::symlink_metadata(target.path()).is_ok() {
            return Err(OpFailure::Taken);
        }
        fs::rename(item.path(), target.path()).map_err(error::io)?;
        self.steps.push(Inverse::RenameBack {
            now: target,
            was: item.name.clone(),
        });
        self.touch_folder(&item.folder);
        Ok(Flow::Go)
    }

    fn trash_items(&mut self, items: &[ItemRef]) -> Step {
        self.progress = Progress::start(items.len(), files_core::ByteSize(0));
        self.emit();
        let thrown = self.trash.trash(items).map_err(error::trash)?;
        if !thrown.is_empty() {
            self.steps.push(Inverse::PutBack(thrown));
        }
        for item in items {
            self.touch_folder(&item.folder);
        }
        self.touch_trash();
        self.progress.items_done = items.len();
        self.emit();
        Ok(Flow::Go)
    }

    fn put_back(&mut self, items: &[TrashedItem]) -> Step {
        self.progress = Progress::start(items.len(), files_core::ByteSize(0));
        self.emit();
        self.trash.put_back(items).map_err(error::trash)?;
        let originals: Vec<ItemRef> = items.iter().map(|item| item.original.clone()).collect();
        for original in &originals {
            self.touch_folder(&original.folder);
        }
        self.touch_trash();
        self.steps.push(Inverse::Trash(originals));
        self.progress.items_done = items.len();
        self.emit();
        Ok(Flow::Go)
    }

    fn delete_for_good(&mut self, items: &[TrashedItem]) -> Step {
        self.trash.delete_for_good(items).map_err(error::trash)?;
        self.touch_trash();
        Ok(Flow::Go)
    }

    fn empty_trash(&mut self) -> Step {
        self.trash.empty().map_err(error::trash)?;
        self.touch_trash();
        Ok(Flow::Go)
    }
}
