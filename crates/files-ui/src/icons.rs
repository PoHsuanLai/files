//! Which icon an item gets: quire's resolver (the person's own change, the chosen pack, the
//! standard set) asked for an entry, and its answer mapped to the choice `ds::FileIcon` draws.
//!
//! Pure: the resolver was built by the edge (`edge::IconSources`) and no file is read here.

use std::sync::Arc;

use ds::prelude::{FileIconChoice, FileKind, ImageSource};
use ds_extensions::{FileFacts, FileIcons, FolderFacts, IconPick};
use ds_helpers::mime::{MimeDb, MimeType, NameGuess};
use files_core::{Entry, EntryKind, FileName, FolderPath};
use files_model::{Opens, opens};

use crate::places::Places;

/// What a file whose name says nothing is.
const UNKNOWN_TYPE: &str = "application/octet-stream";

/// The icons of the window: the resolver for the chosen pack, the standard set it falls back to,
/// the file types and the special folders.
#[derive(Debug, Clone)]
pub(crate) struct Icons {
    db: Arc<MimeDb>,
    resolver: FileIcons,
    standard: FileIcons,
    places: Places,
}

impl Icons {
    /// The icons drawn by `resolver`, for the types of `db`; `places` say which folders are
    /// special (Downloads, Pictures, ...).
    pub(crate) fn new(db: Arc<MimeDb>, resolver: FileIcons, places: Places) -> Icons {
        Icons {
            standard: FileIcons::standard(Arc::clone(&db)),
            db,
            resolver,
            places,
        }
    }

    /// The standard set only.
    pub(crate) fn standard(db: Arc<MimeDb>, places: Places) -> Icons {
        let resolver = FileIcons::standard(Arc::clone(&db));
        Icons::new(db, resolver, places)
    }

    /// The icon of `entry`, which is in `folder` (`None` in the Trash).
    pub(crate) fn choice_for(&self, folder: Option<&FolderPath>, entry: &Entry) -> FileIconChoice {
        let (kind, guess) = match opens(entry) {
            Opens::Folder => {
                let kind = folder
                    .map(|folder| self.places.folder_kind(&folder.join(&entry.name)))
                    .unwrap_or_default();
                (FolderFacts::Folder(kind), MimeType::new(MimeType::FOLDER))
            }
            Opens::File | Opens::Nothing => (FolderFacts::NotAFolder, self.type_of(entry)),
        };
        let Ok(mime) = guess else {
            return other();
        };
        let facts = FileFacts {
            mime: &mime,
            name: entry.shown_name(),
            folder: kind,
        };
        self.mapped(self.resolver.of(&facts), &facts)
    }

    /// The icon of the item `name` of `kind` in `folder`, for what is not a row (Get Info).
    pub(crate) fn choice_of(
        &self,
        folder: &FolderPath,
        name: &FileName,
        kind: EntryKind,
    ) -> FileIconChoice {
        self.choice_for(Some(folder), &Entry::new(name.clone(), kind))
    }

    /// The type of a file: the one the listing found, else what its name says.
    fn type_of(&self, entry: &Entry) -> Result<MimeType, ds_helpers::mime::MimeError> {
        match &entry.mime {
            Some(mime) => Ok(mime.clone()),
            None => match self.db.by_name(entry.shown_name()) {
                NameGuess::Type(mime) => Ok(mime),
                NameGuess::Ambiguous(_) | NameGuess::Unknown => MimeType::new(UNKNOWN_TYPE),
            },
        }
    }

    /// `pick` as the choice `FileIcon` draws; a picture that can't be a `file:` URL is the
    /// standard icon instead.
    fn mapped(&self, pick: IconPick, facts: &FileFacts<'_>) -> FileIconChoice {
        match pick {
            IconPick::Standard { kind, label } => FileIconChoice::Standard { kind, label },
            IconPick::Folder(kind) => FileIconChoice::Folder(kind),
            IconPick::Artwork(path) => ImageSource::file(&path)
                .map_or_else(|_| self.standard_of(facts), FileIconChoice::Artwork),
            // A pick quire adds later is the standard icon until this names it.
            _ => self.standard_of(facts),
        }
    }

    fn standard_of(&self, facts: &FileFacts<'_>) -> FileIconChoice {
        match self.standard.of(facts) {
            IconPick::Standard { kind, label } => FileIconChoice::Standard { kind, label },
            IconPick::Folder(kind) => FileIconChoice::Folder(kind),
            // The standard set draws no artwork; a pick quire adds later is the plain page.
            IconPick::Artwork(_) | _ => other(),
        }
    }
}

/// The plain page, for what has no type to ask about.
fn other() -> FileIconChoice {
    FileIconChoice::Standard {
        kind: FileKind::Other,
        label: None,
    }
}

#[cfg(test)]
mod tests {
    use ds::prelude::FolderKind;
    use ds_helpers::mime::MimeLayer;
    use files_core::ByteSize;

    use super::*;
    use crate::places::PlaceKind;

    fn icons() -> Icons {
        let layer = MimeLayer {
            globs2: "50:application/pdf:*.pdf\n",
            ..MimeLayer::default()
        };
        let places = Places::default().with(
            PlaceKind::Downloads,
            FolderPath::new("/home/p/Downloads").unwrap(),
        );
        Icons::standard(Arc::new(MimeDb::from_layers(&[layer])), places)
    }

    fn entry(name: &str, kind: EntryKind) -> Entry {
        Entry::new(FileName::new(name).unwrap(), kind)
    }

    #[test]
    fn a_file_gets_the_kind_of_its_type_and_a_folder_the_kind_of_its_place() {
        let icons = icons();
        let home = FolderPath::new("/home/p").unwrap();
        let file = EntryKind::File(ByteSize(1));
        // (entry, the kind of the choice)
        let table = [
            (entry("a.pdf", file.clone()), Some(FileKind::Pdf), None),
            (entry("a.nothing", file), Some(FileKind::Other), None),
            (
                entry("Downloads", EntryKind::Folder),
                None,
                Some(FolderKind::Downloads),
            ),
            (
                entry("Other", EntryKind::Folder),
                None,
                Some(FolderKind::Plain),
            ),
        ];
        for (entry, file_kind, folder_kind) in table {
            let choice = icons.choice_for(Some(&home), &entry);
            match (choice, file_kind, folder_kind) {
                (FileIconChoice::Standard { kind, .. }, Some(want), None) => {
                    assert_eq!(kind, want, "{}", entry.name)
                }
                (FileIconChoice::Folder(kind), None, Some(want)) => {
                    assert_eq!(kind, want, "{}", entry.name)
                }
                (other, ..) => panic!("{}: {other:?}", entry.name),
            }
        }
    }

    #[test]
    fn a_folder_in_the_trash_is_a_plain_folder() {
        let choice = icons().choice_for(None, &entry("Downloads", EntryKind::Folder));
        assert_eq!(choice, FileIconChoice::Folder(FolderKind::Plain));
    }
}
