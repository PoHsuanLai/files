//! Putting a colour's tag on files and taking it off, off the window's thread.

use std::collections::BTreeSet;
use std::sync::Arc;

use files_core::{ItemRef, Location, Tag, TagColour};
use files_model::{Change, Input, Mark};

use super::{Edge, Heard};

impl Edge {
    /// Puts the tag of `colour` on `items`, or takes it off. The first file that can't be tagged
    /// ends it with a word to the person; the folders touched, and the place of the colour, are
    /// rescanned either way so the listings show what was done.
    pub(super) fn tag(&self, items: Vec<ItemRef>, colour: TagColour, how: Mark) {
        let (heard, tagger) = (self.heard.clone(), Arc::clone(&self.seams.tagger));
        let started = Edge::work("files-tag", move || {
            let mut touched = BTreeSet::new();
            for item in &items {
                let before = tagger.tags(item);
                let after = match how {
                    Mark::Add => before.clone().with(Tag::of(colour)),
                    Mark::Remove => before.clone().without(colour),
                };
                if after == before {
                    continue;
                }
                match tagger.set(item, &after) {
                    Ok(()) => {
                        touched.insert(item.folder.clone());
                    }
                    Err(failure) => {
                        let _ = heard.send(Heard::Told(failure.to_string()));
                        break;
                    }
                }
            }
            for folder in touched {
                let _ = heard.send(rescan(Location::Folder(folder)));
            }
            let _ = heard.send(rescan(Location::Tag(colour)));
        });
        if started.is_err() {
            self.say(Heard::Told(files_core::OpFailure::Other.to_string()));
        }
    }
}

fn rescan(place: Location) -> Heard {
    Heard::All(Input::Changed(place, Change::Rescan))
}
