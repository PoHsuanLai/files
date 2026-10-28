//! Which thumbnails are wanted, and in what order they are made.
//!
//! A tile that is drawn wants its thumbnail; one that scrolls away does not. Workers take the
//! newest wish first (what the person just scrolled to), at most as many at a time as there are
//! workers, and an answer is delivered only if its tile still wants it. Pure: no thread, no clock.

use super::key::ThumbKey;

/// The wishes, oldest first, each with whether a worker is making it now.
#[derive(Debug, Clone, Default)]
pub struct Queue {
    wishes: Vec<(ThumbKey, bool)>,
}

impl Queue {
    /// A tile wants the thumbnail `key`. Asking again for one already wanted changes nothing.
    pub fn want(&mut self, key: ThumbKey) {
        if self.wishes.iter().all(|(held, _)| *held != key) {
            self.wishes.push((key, false));
        }
    }

    /// The tile that wanted `key` is gone. A thumbnail being made for it is finished and dropped.
    pub fn unwant(&mut self, key: &ThumbKey) {
        self.wishes.retain(|(held, _)| held != key);
    }

    /// The wish to make next: the newest that nobody is making. It is marked as being made.
    pub fn take(&mut self) -> Option<ThumbKey> {
        let wish = self.wishes.iter_mut().rev().find(|(_, making)| !making)?;
        wish.1 = true;
        Some(wish.0.clone())
    }

    /// `key` has been made. The wish ends here; the answer is whether its tile still wants it
    /// (send it on) or was scrolled away (drop it).
    pub fn finish(&mut self, key: &ThumbKey) -> bool {
        let before = self.wishes.len();
        self.unwant(key);
        self.wishes.len() < before
    }

    /// How many wishes are held, waiting or being made.
    pub fn len(&self) -> usize {
        self.wishes.len()
    }

    /// Whether there are none.
    pub fn is_empty(&self) -> bool {
        self.wishes.is_empty()
    }
}
