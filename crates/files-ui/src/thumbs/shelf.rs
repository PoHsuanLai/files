//! The thumbnails the window holds, so a tile that scrolls back into view draws at once.

use std::collections::{HashMap, VecDeque};

use super::key::{ThumbKey, Thumbed};

/// How many answers are kept; the oldest go first.
const KEPT: usize = 512;

/// The answers received, by what was asked.
#[derive(Debug, Clone, Default)]
pub struct Thumbs {
    held: HashMap<ThumbKey, Thumbed>,
    order: VecDeque<ThumbKey>,
}

impl Thumbs {
    /// The answer for `key`, if one has come.
    pub fn get(&self, key: &ThumbKey) -> Option<&Thumbed> {
        self.held.get(key)
    }

    /// Keeps `thumbed` as the answer for `key`, forgetting the oldest answers past the limit.
    pub fn put(&mut self, key: ThumbKey, thumbed: Thumbed) {
        if self.held.insert(key.clone(), thumbed).is_none() {
            self.order.push_back(key);
        }
        while self.order.len() > KEPT {
            if let Some(oldest) = self.order.pop_front() {
                self.held.remove(&oldest);
            }
        }
    }

    /// How many answers are held.
    pub fn len(&self) -> usize {
        self.held.len()
    }

    /// Whether none is held.
    pub fn is_empty(&self) -> bool {
        self.held.is_empty()
    }
}
