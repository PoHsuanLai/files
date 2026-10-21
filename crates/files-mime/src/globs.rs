//! The `globs2` file: which file names mean which type.

use std::collections::HashSet;

use files_core::MimeType;

use crate::fnmatch;
use crate::text::data_lines;

/// What `globs2` writes for "forget the lower folders' patterns for this type".
const NO_GLOBS: &str = "__NOGLOBS__";

/// How strongly a pattern claims a name; the higher wins among patterns as long as each other.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct Weight(pub u32);

/// Whether a pattern tells upper case from lower case.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Case {
    /// `README` also matches `readme`: the default.
    Insensitive,
    /// The pattern was flagged `cs`.
    Sensitive,
}

/// How much of a pattern is a wildcard; the lookup tries them in this order.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Shape {
    Literal,
    Suffix,
    Other,
}

const WILDCARDS: [char; 4] = ['*', '?', '[', '\\'];

fn has_wildcard(text: &str) -> bool {
    text.contains(&WILDCARDS[..])
}

fn shape_of(pattern: &str) -> Shape {
    if !has_wildcard(pattern) {
        return Shape::Literal;
    }
    match pattern.strip_prefix('*') {
        Some(rest) if !rest.is_empty() && !has_wildcard(rest) => Shape::Suffix,
        Some(_) | None => Shape::Other,
    }
}

/// One pattern of `globs2`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct GlobEntry {
    weight: Weight,
    mime: MimeType,
    /// Lower-cased when the entry is case-insensitive.
    pattern: String,
    case: Case,
    shape: Shape,
}

impl GlobEntry {
    /// A pattern claiming `mime` with `weight`.
    pub fn new(weight: Weight, mime: MimeType, pattern: &str, case: Case) -> GlobEntry {
        let pattern = match case {
            Case::Sensitive => pattern.to_owned(),
            Case::Insensitive => pattern.to_lowercase(),
        };
        let shape = shape_of(&pattern);
        GlobEntry {
            weight,
            mime,
            pattern,
            case,
            shape,
        }
    }

    /// The type the pattern names.
    pub fn mime(&self) -> &MimeType {
        &self.mime
    }

    /// The pattern as matched (lower case for a case-insensitive entry).
    pub fn pattern(&self) -> &str {
        &self.pattern
    }

    fn matches(&self, name: &str, folded: &str) -> bool {
        let text = match self.case {
            Case::Sensitive => name,
            Case::Insensitive => folded,
        };
        match self.shape {
            Shape::Literal => text == self.pattern,
            Shape::Suffix => text.ends_with(self.pattern.trim_start_matches('*')),
            Shape::Other => fnmatch::matches(&self.pattern, text),
        }
    }

    fn length(&self) -> usize {
        self.pattern.chars().count()
    }
}

/// What a file name says about the type.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NameGuess {
    /// Exactly one type fits best.
    Type(MimeType),
    /// Several types fit equally well; sorted, with no repeats.
    Ambiguous(Vec<MimeType>),
    /// No pattern fits.
    Unknown,
}

/// All the patterns of one or more `globs2` files.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Globs {
    entries: Vec<GlobEntry>,
    /// Types this file wiped the lower folders' patterns of.
    cleared: HashSet<MimeType>,
}

impl Globs {
    /// The patterns of one `globs2` text. Lines that do not parse are skipped.
    pub fn parse(text: &str) -> Globs {
        let mut globs = Globs::default();
        for line in data_lines(text) {
            let mut fields = line.splitn(4, ':');
            let (Some(weight), Some(mime), Some(pattern)) =
                (fields.next(), fields.next(), fields.next())
            else {
                continue;
            };
            let flags = fields.next().unwrap_or_default();
            let Ok(mime) = MimeType::new(mime) else {
                continue;
            };
            if pattern == NO_GLOBS {
                globs.cleared.insert(mime);
                continue;
            }
            let Ok(weight) = weight.parse::<u32>() else {
                continue;
            };
            let case = if flags.split(',').any(|flag| flag == "cs") {
                Case::Sensitive
            } else {
                Case::Insensitive
            };
            globs
                .entries
                .push(GlobEntry::new(Weight(weight), mime, pattern, case));
        }
        globs
    }

    /// The files in precedence order, highest first, as one set: a higher file's `__NOGLOBS__`
    /// removes that type's patterns from every lower file. The result clears nothing further.
    pub fn merged(layers: impl IntoIterator<Item = Globs>) -> Globs {
        let layers: Vec<Globs> = layers.into_iter().collect();
        let mut entries: Vec<GlobEntry> = Vec::new();
        for layer in layers.into_iter().rev() {
            entries.retain(|entry| !layer.cleared.contains(&entry.mime));
            entries.extend(layer.entries);
        }
        Globs {
            entries,
            cleared: HashSet::new(),
        }
    }

    /// How many patterns there are.
    pub fn len(&self) -> usize {
        self.entries.len()
    }

    /// Whether there is no pattern.
    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    /// The type a file name names: literal patterns first, then `*.ext` ones, then the rest; within
    /// the first group that matches, the longest pattern, then the highest weight.
    pub fn guess(&self, name: &str) -> NameGuess {
        let folded = name.to_lowercase();
        for shape in [Shape::Literal, Shape::Suffix, Shape::Other] {
            let hits: Vec<&GlobEntry> = self
                .entries
                .iter()
                .filter(|entry| entry.shape == shape && entry.matches(name, &folded))
                .collect();
            if !hits.is_empty() {
                return best(&hits);
            }
        }
        NameGuess::Unknown
    }
}

fn best(hits: &[&GlobEntry]) -> NameGuess {
    let longest = hits.iter().map(|entry| entry.length()).max().unwrap_or(0);
    let long: Vec<&&GlobEntry> = hits
        .iter()
        .filter(|entry| entry.length() == longest)
        .collect();
    let top = long.iter().map(|entry| entry.weight).max();
    let mut types: Vec<MimeType> = long
        .iter()
        .filter(|entry| Some(entry.weight) == top)
        .map(|entry| entry.mime.clone())
        .collect();
    types.sort();
    types.dedup();
    match types.len() {
        0 => NameGuess::Unknown,
        1 => types.pop().map_or(NameGuess::Unknown, NameGuess::Type),
        _ => NameGuess::Ambiguous(types),
    }
}
