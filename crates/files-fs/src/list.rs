//! Reading a folder from the disk.

use std::fs::{self, FileType};
use std::os::unix::fs::FileTypeExt;
use std::path::Path;
use std::time::SystemTime;

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, LinkTo, Special};

use crate::error::ListError;

/// Reads folders. Real: [`DiskLister`]; for tests: `FakeLister` (feature `testing`).
pub trait Lister: Send + Sync {
    /// Every entry of `folder`, in no particular order. An entry that vanishes while the folder is
    /// read is left out. The entries carry no type (`mime`); another crate finds it.
    fn list(&self, folder: &FolderPath) -> Result<Vec<Entry>, ListError>;

    /// The one entry `name` of `folder`, as [`Lister::list`] would give it.
    fn entry(&self, folder: &FolderPath, name: &FileName) -> Result<Entry, ListError>;
}

/// How many entries a listing left out because they could not be read or named.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Skipped(pub usize);

/// A folder's entries and how many were left out.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Listing {
    /// What was read.
    pub entries: Vec<Entry>,
    /// What was left out.
    pub skipped: Skipped,
}

/// The real disk.
#[derive(Debug, Clone, Copy, Default)]
pub struct DiskLister;

impl DiskLister {
    /// Like [`Lister::list`], and says how many entries it left out.
    pub fn list_counted(&self, folder: &FolderPath) -> Result<Listing, ListError> {
        let children = fs::read_dir(folder.as_path()).map_err(|e| ListError::from_io(&e))?;
        let mut entries = Vec::new();
        let mut skipped = 0;
        for child in children {
            let entry = child.ok().and_then(|child| {
                let name = FileName::new(child.file_name()).ok()?;
                stat(&child.path(), name).ok()
            });
            match entry {
                Some(entry) => entries.push(entry),
                None => skipped += 1,
            }
        }
        Ok(Listing {
            entries,
            skipped: Skipped(skipped),
        })
    }
}

impl Lister for DiskLister {
    fn list(&self, folder: &FolderPath) -> Result<Vec<Entry>, ListError> {
        self.list_counted(folder).map(|listing| listing.entries)
    }

    fn entry(&self, folder: &FolderPath, name: &FileName) -> Result<Entry, ListError> {
        let path = folder.as_path().join(name.as_os_str());
        stat(&path, name.clone()).map_err(|e| ListError::from_io(&e))
    }
}

/// The entry at `path`, named `name`: links are described, not followed, except to say where they
/// lead.
fn stat(path: &Path, name: FileName) -> std::io::Result<Entry> {
    let meta = fs::symlink_metadata(path)?;
    let kind = kind_of(path, meta.file_type(), ByteSize(meta.len()));
    Ok(with_time(Entry::new(name, kind), meta.modified().ok()))
}

fn with_time(entry: Entry, modified: Option<SystemTime>) -> Entry {
    match modified {
        Some(at) => entry.modified_at(at),
        None => entry,
    }
}

fn kind_of(path: &Path, file_type: FileType, size: ByteSize) -> EntryKind {
    if file_type.is_symlink() {
        EntryKind::Link(link_to(path))
    } else if file_type.is_dir() {
        EntryKind::Folder
    } else if file_type.is_file() {
        EntryKind::File(size)
    } else {
        EntryKind::Special(special(file_type))
    }
}

/// What the link at `path` leads to. Anything that is not a folder counts as a file (a link to a
/// device opens like one); a target that can't be reached is broken.
fn link_to(path: &Path) -> LinkTo {
    match fs::metadata(path) {
        Ok(target) if target.is_dir() => LinkTo::Folder,
        Ok(target) => LinkTo::File(ByteSize(target.len())),
        Err(_) => LinkTo::Broken,
    }
}

fn special(file_type: FileType) -> Special {
    if file_type.is_fifo() {
        Special::Fifo
    } else if file_type.is_socket() {
        Special::Socket
    } else {
        Special::Device
    }
}
