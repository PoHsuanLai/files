//! Tags: the names a person gives a file, and the seven colours Finder offers.
//!
//! A file's tags are kept as a comma-separated list in the `user.xdg.tags` extended attribute, as
//! GNOME and KDE keep them. A tag named like a colour (`Red`) is that colour's tag; any other name
//! is a tag of the person's own, kept and shown by name.

use std::fmt;

use crate::TagError;

/// One of Finder's seven tag colours.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
#[non_exhaustive]
pub enum TagColour {
    /// Red.
    Red,
    /// Orange.
    Orange,
    /// Yellow.
    Yellow,
    /// Green.
    Green,
    /// Blue.
    Blue,
    /// Purple.
    Purple,
    /// Gray.
    Gray,
}

impl TagColour {
    /// The seven colours, in Finder's order.
    pub const ALL: [TagColour; 7] = [
        TagColour::Red,
        TagColour::Orange,
        TagColour::Yellow,
        TagColour::Green,
        TagColour::Blue,
        TagColour::Purple,
        TagColour::Gray,
    ];

    /// The colour's name, which is also the name of its tag.
    pub fn label(self) -> &'static str {
        match self {
            TagColour::Red => "Red",
            TagColour::Orange => "Orange",
            TagColour::Yellow => "Yellow",
            TagColour::Green => "Green",
            TagColour::Blue => "Blue",
            TagColour::Purple => "Purple",
            TagColour::Gray => "Gray",
        }
    }

    /// The colour a name stands for, ignoring case; `None` for any other name.
    pub fn named(name: &str) -> Option<TagColour> {
        TagColour::ALL
            .into_iter()
            .find(|colour| colour.label().eq_ignore_ascii_case(name))
    }
}

/// One tag: a name with no comma and no control character, trimmed.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct Tag(String);

impl Tag {
    /// The tag called `name`, when it is a name a tag can have. Spaces around it are dropped.
    pub fn new(name: &str) -> Result<Tag, TagError> {
        let name = name.trim();
        if name.is_empty() {
            Err(TagError::Empty)
        } else if name.contains(',') {
            Err(TagError::Comma)
        } else if name.chars().any(char::is_control) {
            Err(TagError::Control)
        } else {
            Ok(Tag(name.to_owned()))
        }
    }

    /// The tag of a colour.
    pub fn of(colour: TagColour) -> Tag {
        Tag(colour.label().to_owned())
    }

    /// The tag's name.
    pub fn name(&self) -> &str {
        &self.0
    }

    /// The colour the tag is, when its name is one of the seven.
    pub fn colour(&self) -> Option<TagColour> {
        TagColour::named(&self.0)
    }
}

impl fmt::Display for Tag {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.0)
    }
}

/// The tags of one file, in the order they were given, each once.
#[derive(Debug, Clone, PartialEq, Eq, Hash, Default)]
pub struct Tags(Vec<Tag>);

impl Tags {
    /// No tags.
    pub fn none() -> Tags {
        Tags::default()
    }

    /// The tags in `attribute`, a comma-separated list. A name that is not a tag is skipped, and a
    /// tag named twice is kept once.
    pub fn parse(attribute: &str) -> Tags {
        attribute
            .split(',')
            .filter_map(|name| Tag::new(name).ok())
            .fold(Tags::none(), Tags::with)
    }

    /// The comma-separated list the attribute holds.
    pub fn to_attribute(&self) -> String {
        let names: Vec<&str> = self.0.iter().map(Tag::name).collect();
        names.join(",")
    }

    /// The tags, in order.
    pub fn iter(&self) -> impl Iterator<Item = &Tag> {
        self.0.iter()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// How many there are.
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Whether `tag` is among them.
    pub fn contains(&self, tag: &Tag) -> bool {
        self.0.contains(tag)
    }

    /// Whether the tag of `colour` is among them.
    pub fn has(&self, colour: TagColour) -> bool {
        self.0.iter().any(|tag| tag.colour() == Some(colour))
    }

    /// The colours among them, in the order they were given.
    pub fn colours(&self) -> impl Iterator<Item = TagColour> + '_ {
        self.0.iter().filter_map(Tag::colour)
    }

    /// These tags and `tag`, added at the end unless already there.
    pub fn with(mut self, tag: Tag) -> Tags {
        if !self.0.contains(&tag) {
            self.0.push(tag);
        }
        self
    }

    /// These tags without the tag of `colour`.
    pub fn without(mut self, colour: TagColour) -> Tags {
        self.0.retain(|tag| tag.colour() != Some(colour));
        self
    }

    /// These tags with the tag of `colour` added when it was not there and removed when it was.
    pub fn toggled(self, colour: TagColour) -> Tags {
        if self.has(colour) {
            self.without(colour)
        } else {
            self.with(Tag::of(colour))
        }
    }
}
