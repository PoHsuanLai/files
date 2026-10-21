//! A scratch folder, a fake Trash and a host that answers from a script.

use std::collections::VecDeque;
use std::fs;
use std::path::Path;
use std::sync::Arc;

use files_core::{Answer, Conflict, FileName, FolderPath, ItemRef, Progress, Resolution, Transfer};
use files_ops::{Executor, Flow, Host, Journal};
use files_trash::FakeTrash;
use tempfile::TempDir;

/// A scratch folder with `home/` (where the files are), `dest/` (an empty folder to bring them
/// to) and `bin/` (the fake Trash), and an executor over it.
pub struct Rig {
    pub dir: TempDir,
    pub exec: Executor,
    pub trash: Arc<FakeTrash>,
    pub journal: Journal,
}

pub fn rig() -> Rig {
    let dir = tempfile::tempdir().unwrap();
    fs::create_dir(dir.path().join("home")).unwrap();
    fs::create_dir(dir.path().join("dest")).unwrap();
    let trash = Arc::new(FakeTrash::new(dir.path().join("bin")));
    let exec = Executor::new(trash.clone());
    Rig {
        dir,
        exec,
        trash,
        journal: Journal::new(),
    }
}

pub fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

impl Rig {
    pub fn path(&self, relative: &str) -> std::path::PathBuf {
        self.dir.path().join(relative)
    }

    pub fn folder(&self, relative: &str) -> FolderPath {
        FolderPath::new(self.path(relative)).unwrap()
    }

    /// The item `relative` (like `home/a.txt`).
    pub fn item(&self, relative: &str) -> ItemRef {
        ItemRef::from_path(&self.path(relative)).unwrap()
    }

    /// Writes `text` to `relative`, making its folder if need be.
    pub fn write(&self, relative: &str, text: &str) -> ItemRef {
        let path = self.path(relative);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, text).unwrap();
        self.item(relative)
    }

    pub fn read(&self, relative: &str) -> String {
        fs::read_to_string(self.path(relative)).unwrap()
    }

    pub fn exists(&self, relative: &str) -> bool {
        fs::symlink_metadata(self.path(relative)).is_ok()
    }

    /// The names in `relative`, sorted.
    pub fn names(&self, relative: &str) -> Vec<String> {
        let mut names: Vec<String> = fs::read_dir(self.path(relative))
            .unwrap()
            .map(|e| e.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        names
    }

    /// Brings `sources` to `dest/`.
    pub fn transfer(&self, sources: &[&str]) -> Transfer {
        let sources = sources.iter().map(|s| self.item(s)).collect();
        Transfer::new(sources, self.folder("dest"))
    }
}

/// A host that answers conflicts from a list, remembers what it was asked and told, and stops
/// after a number of progress reports when it is told to.
#[derive(Default)]
pub struct Script {
    pub answers: VecDeque<Resolution>,
    pub asked: Vec<Conflict>,
    pub progress: Vec<Progress>,
    pub stop_after: Option<usize>,
}

impl Script {
    pub fn answering(answers: &[Resolution]) -> Script {
        Script {
            answers: answers.iter().copied().collect(),
            ..Script::default()
        }
    }

    pub fn stopping_after(reports: usize) -> Script {
        Script {
            stop_after: Some(reports),
            ..Script::default()
        }
    }
}

impl Host for Script {
    fn progress(&mut self, progress: Progress) {
        self.progress.push(progress);
    }

    fn ask(&mut self, conflict: &Conflict) -> Resolution {
        self.asked.push(conflict.clone());
        self.answers
            .pop_front()
            .unwrap_or(Resolution::once(Answer::Stop))
    }

    fn flow(&self) -> Flow {
        match self.stop_after {
            Some(limit) if self.progress.len() >= limit => Flow::Stop,
            Some(_) | None => Flow::Go,
        }
    }
}

/// `path` as the file it is, for symlink checks.
pub fn is_link(path: &Path) -> bool {
    fs::symlink_metadata(path).unwrap().file_type().is_symlink()
}
