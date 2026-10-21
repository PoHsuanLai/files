//! Finder's naming for a second item that keeps the first: `report.pdf` becomes `report 2.pdf`.

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};

use crate::{FileName, NameError};

/// Whether a name's extension stays at the end when a number is added.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Split {
    /// `report.pdf` becomes `report 2.pdf`: for files.
    Extension,
    /// `v1.2` becomes `v1.2 2`: for folders, whose dots are part of the name.
    Whole,
}

/// `name` itself when `taken` does not hold it, else the first of `name 2`, `name 3`, ... that
/// `taken` does not hold. Fails only when the number would make the name too long.
pub fn keep_both(
    name: &FileName,
    split: Split,
    taken: impl Fn(&FileName) -> bool,
) -> Result<FileName, NameError> {
    if !taken(name) {
        return Ok(name.clone());
    }
    let (stem, extension) = match split {
        Split::Extension => (name.stem(), name.extension()),
        Split::Whole => (name.as_os_str(), None),
    };
    let mut number: u64 = 2;
    loop {
        let mut bytes = stem.as_bytes().to_vec();
        bytes.extend_from_slice(format!(" {number}").as_bytes());
        if let Some(extension) = extension {
            bytes.push(b'.');
            bytes.extend_from_slice(extension.as_bytes());
        }
        let candidate = FileName::new(OsString::from_vec(bytes))?;
        if !taken(&candidate) {
            return Ok(candidate);
        }
        number += 1;
    }
}
