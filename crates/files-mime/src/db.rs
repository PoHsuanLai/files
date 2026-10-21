//! The shared-mime-info database: names to types, aliases, subclasses and icons.

use std::collections::VecDeque;

use files_core::{FileName, MimeType};

use crate::globs::{Globs, NameGuess};
use crate::tables::{Aliases, IconNames, Subclasses};

const PLAIN_TEXT: &str = "text/plain";
const OCTET_STREAM: &str = "application/octet-stream";

fn known(text: &str) -> Option<MimeType> {
    MimeType::new(text).ok()
}

/// The texts of one data folder's `mime/` files; a missing file is the empty text.
#[derive(Debug, Clone, Copy, Default)]
pub struct MimeLayer<'a> {
    /// `globs2`.
    pub globs2: &'a str,
    /// `aliases`.
    pub aliases: &'a str,
    /// `subclasses`.
    pub subclasses: &'a str,
    /// `icons`.
    pub icons: &'a str,
    /// `generic-icons`.
    pub generic_icons: &'a str,
}

/// Whether one type is a kind of another.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Relation {
    /// It is the same type or a special case of it.
    Is,
    /// It is not.
    IsNot,
}

/// The icon names to try for a type, in order: the specific one, then the generic one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct IconName {
    specific: String,
    generic: String,
}

impl IconName {
    /// The type's own icon: `application-pdf`.
    pub fn specific(&self) -> &str {
        &self.specific
    }

    /// The icon of the type's kind: `x-office-document`, or `image-x-generic`.
    pub fn generic(&self) -> &str {
        &self.generic
    }

    /// Both names, the one to try first first.
    pub fn in_order(&self) -> [&str; 2] {
        [&self.specific, &self.generic]
    }
}

/// Everything the database knows, merged across data folders.
#[derive(Debug, Clone, Default)]
pub struct MimeDb {
    globs: Globs,
    aliases: Aliases,
    subclasses: Subclasses,
    icons: IconNames,
    generic_icons: IconNames,
}

impl MimeDb {
    /// The database of the folders' texts, in precedence order: earlier layers win.
    pub fn from_layers(layers: &[MimeLayer<'_>]) -> MimeDb {
        MimeDb {
            globs: Globs::merged(layers.iter().map(|l| Globs::parse(l.globs2))),
            aliases: Aliases::merged(layers.iter().map(|l| Aliases::parse(l.aliases))),
            subclasses: Subclasses::merged(layers.iter().map(|l| Subclasses::parse(l.subclasses))),
            icons: IconNames::merged(layers.iter().map(|l| IconNames::parse(l.icons))),
            generic_icons: IconNames::merged(
                layers.iter().map(|l| IconNames::parse(l.generic_icons)),
            ),
        }
    }

    /// What a file's name says about its type. Content is not read; see the crate docs.
    pub fn by_name(&self, name: &FileName) -> NameGuess {
        self.globs.guess(&name.display())
    }

    /// `mime` with an alias resolved to the type it stands for.
    pub fn canonical(&self, mime: &MimeType) -> MimeType {
        self.aliases.canonical(mime)
    }

    /// Every type `mime` is a special case of, nearest first: its subclass parents breadth first
    /// (aliases resolved), `text/plain` for a text type, and `application/octet-stream` last for
    /// any type that is not `inode/*`.
    pub fn ancestors(&self, mime: &MimeType) -> Vec<MimeType> {
        let start = self.canonical(mime);
        let mut found: Vec<MimeType> = Vec::new();
        let mut queue = VecDeque::from([start.clone()]);
        while let Some(current) = queue.pop_front() {
            for parent in self.direct_parents(&current) {
                if parent != start && !found.contains(&parent) {
                    found.push(parent.clone());
                    queue.push_back(parent);
                }
            }
        }
        if start.major() != "inode" && start.as_str() != OCTET_STREAM {
            if let Some(octet) = known(OCTET_STREAM) {
                if !found.contains(&octet) {
                    found.push(octet);
                }
            }
        }
        found
    }

    fn direct_parents(&self, mime: &MimeType) -> Vec<MimeType> {
        let mut parents: Vec<MimeType> = self
            .subclasses
            .parents_of(mime)
            .iter()
            .map(|parent| self.canonical(parent))
            .collect();
        if mime.major() == "text" && mime.as_str() != PLAIN_TEXT {
            if let Some(plain) = known(PLAIN_TEXT) {
                if !parents.contains(&plain) {
                    parents.push(plain);
                }
            }
        }
        parents
    }

    /// Whether `child` is `parent` or a special case of it, through subclasses and the implicit
    /// rules (every `text/*` is `text/plain`; every type but `inode/*` is `application/octet-stream`).
    pub fn is_a(&self, child: &MimeType, parent: &MimeType) -> Relation {
        let parent = self.canonical(parent);
        if self.canonical(child) == parent || self.ancestors(child).contains(&parent) {
            Relation::Is
        } else {
            Relation::IsNot
        }
    }

    /// The icon names for `mime`: its icon (or the type with `/` as `-`) and its generic one (or
    /// `<major>-x-generic`).
    pub fn icon_name(&self, mime: &MimeType) -> IconName {
        let mime = self.canonical(mime);
        let specific = self
            .icons
            .get(&mime)
            .map_or_else(|| mime.as_str().replace('/', "-"), str::to_owned);
        let generic = self
            .generic_icons
            .get(&mime)
            .map_or_else(|| format!("{}-x-generic", mime.major()), str::to_owned);
        IconName { specific, generic }
    }
}
