//! Which thumbnails are wanted, and in what order they are made.
//!
//! A tile that is drawn wants its thumbnail; one that scrolls away does not. Workers take the
//! newest wish first (what the person just scrolled to), at most as many at a time as there are
//! workers, and an answer is delivered only if its tile still wants it. Pure: no thread, no clock.

use super::key::ThumbKey;

/// Where a wish stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Stage {
    Waiting,
    Making,
}

/// What to do with a finished thumbnail.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Delivery {
    /// Its tile still wants it: send it on.
    Deliver,
    /// Its tile was scrolled away (or never wanted it): drop it.
    Drop,
}

/// The wishes, oldest first.
#[derive(Debug, Clone, Default)]
pub struct Queue {
    wishes: Vec<(ThumbKey, Stage)>,
}

impl Queue {
    /// A tile wants the thumbnail `key`. Asking again for one already wanted changes nothing.
    pub fn want(&mut self, key: ThumbKey) {
        if self.wishes.iter().all(|(held, _)| *held != key) {
            self.wishes.push((key, Stage::Waiting));
        }
    }

    /// The tile that wanted `key` is gone. A thumbnail being made for it is finished and dropped.
    pub fn unwant(&mut self, key: &ThumbKey) {
        self.wishes.retain(|(held, _)| held != key);
    }

    /// The wish to make next: the newest that nobody is making. It is marked as being made.
    pub fn take(&mut self) -> Option<ThumbKey> {
        let wish = self
            .wishes
            .iter_mut()
            .rev()
            .find(|(_, stage)| *stage == Stage::Waiting)?;
        wish.1 = Stage::Making;
        Some(wish.0.clone())
    }

    /// `key` has been made. The wish ends here: whether to deliver it depends on its tile still
    /// wanting it.
    pub fn finish(&mut self, key: &ThumbKey) -> Delivery {
        let before = self.wishes.len();
        self.unwant(key);
        if self.wishes.len() < before {
            Delivery::Deliver
        } else {
            Delivery::Drop
        }
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
