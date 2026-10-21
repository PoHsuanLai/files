//! The real tagger: the `user.xdg.tags` extended attribute.

use files_core::{ItemRef, Tags};

use super::Tagger;
use crate::error::TagWriteError;

/// The extended attribute that holds a file's tags.
pub const ATTRIBUTE: &str = "user.xdg.tags";

/// Tags kept in [`ATTRIBUTE`]. A link's tags are its target's.
#[derive(Debug, Clone, Copy, Default)]
pub struct XattrTagger;

impl Tagger for XattrTagger {
    fn tags(&self, item: &ItemRef) -> Tags {
        match xattr::get_deref(item.path(), ATTRIBUTE) {
            Ok(Some(bytes)) => Tags::parse(&String::from_utf8_lossy(&bytes)),
            Ok(None) | Err(_) => Tags::none(),
        }
    }

    fn set(&self, item: &ItemRef, tags: &Tags) -> Result<(), TagWriteError> {
        let path = item.path();
        let done = if tags.is_empty() {
            match xattr::remove_deref(&path, ATTRIBUTE) {
                // Nothing to remove is the state asked for.
                Err(err) if err.raw_os_error() == Some(NO_ATTRIBUTE) => Ok(()),
                other => other,
            }
        } else {
            xattr::set_deref(&path, ATTRIBUTE, tags.to_attribute().as_bytes())
        };
        done.map_err(|err| TagWriteError::from_io(&err))
    }
}

/// `ENODATA`: the file has no such attribute.
const NO_ATTRIBUTE: i32 = 61;
