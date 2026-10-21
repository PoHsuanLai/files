//! Copying and removing whole trees of files, without ever following a link.

use std::fs::{self, File, OpenOptions};
use std::io::{self, Read, Write};
use std::os::unix::fs::symlink;
use std::path::Path;

use files_core::OpFailure;

use super::{Next, Run, Step};
use crate::error;
use crate::host::Flow;

/// How much of a file is read and written at a time; progress and cancel are checked between.
const CHUNK: usize = 256 * 1024;

impl Run<'_> {
    /// Copies `from` to `to`, which must not exist. A link is copied as a link, a folder with
    /// everything in it, a file with its permissions and modification time; a pipe, a socket or
    /// a device is left out. Stops between chunks when the host says so; the caller removes what
    /// was half made.
    pub(super) fn copy_tree(&mut self, from: &Path, to: &Path) -> Step {
        let meta = fs::symlink_metadata(from).map_err(error::io)?;
        let kind = meta.file_type();
        if kind.is_symlink() {
            let target = fs::read_link(from).map_err(error::io)?;
            symlink(target, to).map_err(error::io)?;
            return Ok(Next::Go);
        }
        if kind.is_dir() {
            fs::create_dir(to).map_err(error::io)?;
            for child in fs::read_dir(from).map_err(error::io)? {
                let child = child.map_err(error::io)?;
                let step = self.copy_tree(&child.path(), &to.join(child.file_name()))?;
                if step == Next::Stop {
                    return Ok(Next::Stop);
                }
            }
            // After the contents, so a read-only folder can still be filled.
            let _ = fs::set_permissions(to, meta.permissions());
            return Ok(Next::Go);
        }
        if kind.is_file() {
            return self.copy_file(from, to, &meta);
        }
        Ok(Next::Go)
    }

    fn copy_file(&mut self, from: &Path, to: &Path, meta: &fs::Metadata) -> Step {
        let mut input = File::open(from).map_err(error::io)?;
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(to)
            .map_err(error::io)?;
        let mut buffer = vec![0u8; CHUNK];
        loop {
            if self.host.flow() == Flow::Stop {
                return Ok(Next::Stop);
            }
            let read = match input.read(&mut buffer) {
                Ok(0) => break,
                Ok(read) => read,
                Err(err) if err.kind() == io::ErrorKind::Interrupted => continue,
                Err(err) => return Err(error::io(err)),
            };
            output.write_all(&buffer[..read]).map_err(error::io)?;
            self.progress = self.progress.wrote(read as u64);
            self.emit();
        }
        // Best effort: a copy that cannot take the original's mode or date is still a copy.
        let _ = output.set_permissions(meta.permissions());
        if let Ok(modified) = meta.modified() {
            let _ = output.set_modified(modified);
        }
        Ok(Next::Go)
    }

    /// Moves `from` to `to`, which must not exist: a rename, or when the two are on different
    /// disks a copy followed by removing the original.
    pub(super) fn move_path(&mut self, from: &Path, to: &Path) -> Step {
        match fs::rename(from, to) {
            Ok(()) => Ok(Next::Go),
            Err(err) if err.kind() == io::ErrorKind::CrossesDevices => self.move_across(from, to),
            Err(err) => Err(error::io(err)),
        }
    }

    fn move_across(&mut self, from: &Path, to: &Path) -> Step {
        match self.copy_tree(from, to) {
            Ok(Next::Go) => remove_tree(from).map(|()| Next::Go),
            Ok(Next::Stop) => {
                let _ = remove_tree(to);
                Ok(Next::Stop)
            }
            Err(failure) => {
                let _ = remove_tree(to);
                Err(failure)
            }
        }
    }
}

/// Removes the file, link or folder at `path` with everything in it; a link is removed, not
/// followed.
pub(super) fn remove_tree(path: &Path) -> Result<(), OpFailure> {
    let meta = fs::symlink_metadata(path).map_err(error::io)?;
    let removed = if meta.is_dir() {
        fs::remove_dir_all(path)
    } else {
        fs::remove_file(path)
    };
    removed.map_err(error::io)
}

/// The bytes in the files under `path`, links not followed; what cannot be read counts as none.
pub(super) fn bytes_under(path: &Path) -> u64 {
    let Ok(meta) = fs::symlink_metadata(path) else {
        return 0;
    };
    if meta.is_dir() {
        let Ok(children) = fs::read_dir(path) else {
            return 0;
        };
        children
            .filter_map(Result::ok)
            .map(|child| bytes_under(&child.path()))
            .sum()
    } else if meta.is_file() {
        meta.len()
    } else {
        0
    }
}
