//! One thing in a folder, as a listing found it.

use std::time::SystemTime;

use crate::{ByteSize, FileName, ItemRef, MimeType, Tags, TrashedItem};

/// One entry of a folder: its name, what it is, and what the listing knew about it.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Entry {
    /// Its name in the folder.
    pub name: FileName,
    /// What it is, with what only that kind has (a file's size, a link's target).
    pub kind: EntryKind,
    /// When its contents last changed; `None` when the filesystem does not say.
    pub modified: Option<SystemTime>,
    /// Its type, when it has been found (the listing may leave it for later).
    pub mime: Option<MimeType>,
    /// For a row of the Trash: the trashed item it stands for. Its `name` is then the name the
    /// item has inside the Trash (unique there); [`Entry::shown_name`] is the one a person reads.
    pub trashed: Option<TrashedItem>,
    /// The tags the file carries; none until the caller reads them.
    pub tags: Tags,
    /// For a row of a search: the item it stands for, wherever it is. Its `name` is then a key
    /// made from the whole path (unique in the results); [`Entry::shown_name`] is the name a
    /// person reads.
    pub found: Option<ItemRef>,
}

impl Entry {
    /// An entry named `name` of `kind`, with nothing else known yet.
    pub fn new(name: FileName, kind: EntryKind) -> Entry {
        Entry {
            name,
            kind,
            modified: None,
            mime: None,
            trashed: None,
            tags: Tags::none(),
            found: None,
        }
    }

    /// The same entry carrying `tags`.
    pub fn with_tags(self, tags: Tags) -> Entry {
        Entry { tags, ..self }
    }

    /// The same entry as a row of a search, standing for `item`. The row is keyed by the whole
    /// path (its slashes written as U+2215), so two files of one name in different folders are
    /// two rows, and the key is the same every time the search is run.
    pub fn found_at(self, item: ItemRef) -> Entry {
        let key: String = item
            .path()
            .to_string_lossy()
            .trim_start_matches('/')
            .replace('/', "\u{2215}");
        let name = FileName::new(key).unwrap_or_else(|_| item.name.clone());
        Entry {
            name,
            found: Some(item),
            ..self
        }
    }

    /// The same entry, standing for the trashed `item`.
    pub fn in_trash(self, item: TrashedItem) -> Entry {
        Entry {
            trashed: Some(item),
            ..self
        }
    }

    /// The name a person reads: the original name of a trashed item or the name of a found one, else
    /// the entry's own.
    pub fn shown_name(&self) -> &FileName {
        match (&self.trashed, &self.found) {
            (Some(item), _) => &item.original.name,
            (None, Some(item)) => &item.name,
            (None, None) => &self.name,
        }
    }

    /// The same entry, modified at `at`.
    pub fn modified_at(self, at: SystemTime) -> Entry {
        Entry {
            modified: Some(at),
            ..self
        }
    }

    /// The same entry, of type `mime`.
    pub fn with_mime(self, mime: MimeType) -> Entry {
        Entry {
            mime: Some(mime),
            ..self
        }
    }

    /// Whether a person sees it by default: a name that starts with a dot is hidden.
    pub fn visibility(&self) -> Visibility {
        self.name.visibility()
    }
}

/// What an entry is.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum EntryKind {
    /// A regular file of this size.
    File(ByteSize),
    /// A folder.
    Folder,
    /// A symbolic link, and what it leads to.
    Link(LinkTo),
    /// Something that is neither file nor folder.
    Special(Special),
}

/// What a symbolic link leads to.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum LinkTo {
    /// A file of this size.
    File(ByteSize),
    /// A folder; opening the link opens it.
    Folder,
    /// Nothing: its target is gone or cannot be reached.
    Broken,
}

/// A filesystem object that holds no contents a person opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub enum Special {
    /// A named pipe.
    Fifo,
    /// A socket.
    Socket,
    /// A character or block device.
    Device,
}

/// Whether an entry is shown when hidden files are not.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Visibility {
    /// Shown by default.
    Shown,
    /// Shown only while hidden files are (its name starts with a dot).
    Hidden,
}
