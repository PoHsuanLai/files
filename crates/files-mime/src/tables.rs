//! The small tables of the database: aliases, subclasses and icon names.

use std::collections::HashMap;

use files_core::MimeType;

use crate::text::data_lines;

/// `alias canonical` lines: other names a type goes by.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Aliases {
    to_canonical: HashMap<MimeType, MimeType>,
}

impl Aliases {
    /// The aliases of one `aliases` text.
    pub fn parse(text: &str) -> Aliases {
        let to_canonical = data_lines(text)
            .filter_map(|line| {
                let mut words = line.split_whitespace();
                let alias = MimeType::new(words.next()?).ok()?;
                let canonical = MimeType::new(words.next()?).ok()?;
                Some((alias, canonical))
            })
            .collect();
        Aliases { to_canonical }
    }

    /// The files in precedence order, highest first; the first to name an alias decides it.
    pub fn merged(layers: impl IntoIterator<Item = Aliases>) -> Aliases {
        let mut to_canonical = HashMap::new();
        for layer in layers {
            for (alias, canonical) in layer.to_canonical {
                to_canonical.entry(alias).or_insert(canonical);
            }
        }
        Aliases { to_canonical }
    }

    /// The type `mime` is an alias of, or `mime` itself.
    pub fn canonical(&self, mime: &MimeType) -> MimeType {
        self.to_canonical.get(mime).unwrap_or(mime).clone()
    }
}

/// `sub parent` lines: a type that is a special case of another.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Subclasses {
    parents: HashMap<MimeType, Vec<MimeType>>,
}

impl Subclasses {
    /// The relations of one `subclasses` text.
    pub fn parse(text: &str) -> Subclasses {
        let mut parents: HashMap<MimeType, Vec<MimeType>> = HashMap::new();
        for line in data_lines(text) {
            let mut words = line.split_whitespace();
            let (Some(sub), Some(parent)) = (words.next(), words.next()) else {
                continue;
            };
            let (Ok(sub), Ok(parent)) = (MimeType::new(sub), MimeType::new(parent)) else {
                continue;
            };
            parents.entry(sub).or_default().push(parent);
        }
        Subclasses { parents }
    }

    /// The files in precedence order, highest first; a type's parents are all files' parents,
    /// the higher files' first.
    pub fn merged(layers: impl IntoIterator<Item = Subclasses>) -> Subclasses {
        let mut parents: HashMap<MimeType, Vec<MimeType>> = HashMap::new();
        for layer in layers {
            for (sub, list) in layer.parents {
                let known = parents.entry(sub).or_default();
                for parent in list {
                    if !known.contains(&parent) {
                        known.push(parent);
                    }
                }
            }
        }
        Subclasses { parents }
    }

    /// The types `mime` is directly a special case of, as written (aliases not resolved).
    pub fn parents_of(&self, mime: &MimeType) -> &[MimeType] {
        self.parents
            .get(mime)
            .map(Vec::as_slice)
            .unwrap_or_default()
    }
}

/// `type:icon-name` lines (the `icons` and `generic-icons` files).
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct IconNames {
    names: HashMap<MimeType, String>,
}

impl IconNames {
    /// The names of one `icons` or `generic-icons` text.
    pub fn parse(text: &str) -> IconNames {
        let names = data_lines(text)
            .filter_map(|line| {
                let (mime, icon) = line.split_once(':')?;
                let icon = icon.trim();
                if icon.is_empty() {
                    return None;
                }
                Some((MimeType::new(mime.trim()).ok()?, icon.to_owned()))
            })
            .collect();
        IconNames { names }
    }

    /// The files in precedence order, highest first; the first to name a type decides it.
    pub fn merged(layers: impl IntoIterator<Item = IconNames>) -> IconNames {
        let mut names = HashMap::new();
        for layer in layers {
            for (mime, icon) in layer.names {
                names.entry(mime).or_insert(icon);
            }
        }
        IconNames { names }
    }

    /// The icon named for `mime`, if any.
    pub fn get(&self, mime: &MimeType) -> Option<&str> {
        self.names.get(mime).map(String::as_str)
    }
}
