//! Which programs have a disk open, found from `/proc`, behind a trait so a test gives its own.

use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};

use files_core::ProgramName;

/// Finds the programs that have something under a mount point open.
pub trait DiskUsers: Send + Sync {
    /// The programs using `mount`, by name, without repeats; empty when none was found or the
    /// search could not be made. Reads only what this person may read.
    fn using(&self, mount: &Path) -> Vec<ProgramName>;
}

/// Finds nobody: the window says only that programs may be using the disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoUsers;

impl DiskUsers for NoUsers {
    fn using(&self, _mount: &Path) -> Vec<ProgramName> {
        Vec::new()
    }
}

/// Reads the process folder `root` (`/proc`): each process's open files and working folder.
#[derive(Debug, Clone)]
pub struct ProcUsers {
    root: PathBuf,
}

impl ProcUsers {
    /// Searches the processes listed under `root`.
    pub fn new(root: impl Into<PathBuf>) -> ProcUsers {
        ProcUsers { root: root.into() }
    }

    /// Whether the process folder `process` has something under `mount` open or is working there.
    fn holds(process: &Path, mount: &Path) -> bool {
        let inside =
            |link: PathBuf| fs::read_link(link).is_ok_and(|target| target.starts_with(mount));
        if inside(process.join("cwd")) {
            return true;
        }
        let Ok(fds) = fs::read_dir(process.join("fd")) else {
            return false;
        };
        fds.filter_map(Result::ok).any(|fd| inside(fd.path()))
    }

    /// The process's own name, as the kernel keeps it.
    fn name_of(process: &Path) -> Option<ProgramName> {
        let text = fs::read_to_string(process.join("comm")).ok()?;
        let name = text.trim();
        (!name.is_empty()).then(|| ProgramName::new(name))
    }
}

impl DiskUsers for ProcUsers {
    fn using(&self, mount: &Path) -> Vec<ProgramName> {
        let Ok(processes) = fs::read_dir(&self.root) else {
            return Vec::new();
        };
        let names: BTreeSet<ProgramName> = processes
            .filter_map(Result::ok)
            .map(|entry| entry.path())
            .filter(|process| ProcUsers::holds(process, mount))
            .filter_map(|process| ProcUsers::name_of(&process))
            .collect();
        names.into_iter().collect()
    }
}

/// Finds the programs it was given, whatever the mount.
#[cfg(feature = "testing")]
#[derive(Debug, Clone, Default)]
pub struct FakeUsers(pub Vec<ProgramName>);

#[cfg(feature = "testing")]
impl DiskUsers for FakeUsers {
    fn using(&self, _mount: &Path) -> Vec<ProgramName> {
        self.0.clone()
    }
}
