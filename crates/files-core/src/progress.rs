//! How far a piece of work has come.

use crate::ByteSize;

/// Items and bytes done out of the total.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
#[non_exhaustive]
pub struct Progress {
    /// Items finished.
    pub items_done: usize,
    /// Items in all.
    pub items_total: usize,
    /// Bytes written so far.
    pub bytes_done: ByteSize,
    /// Bytes to write in all.
    pub bytes_total: ByteSize,
}

impl Progress {
    /// Nothing done yet, out of `items_total` items and `bytes_total` bytes.
    pub fn start(items_total: usize, bytes_total: ByteSize) -> Progress {
        Progress {
            items_done: 0,
            items_total,
            bytes_done: ByteSize(0),
            bytes_total,
        }
    }

    /// How far along, in thousandths: by bytes when there are any to write, else by items. A
    /// piece of work with nothing to do is whole.
    pub fn permille(&self) -> u16 {
        let (done, total) = if self.bytes_total.0 > 0 {
            (self.bytes_done.0, self.bytes_total.0)
        } else {
            (self.items_done as u64, self.items_total as u64)
        };
        if total == 0 {
            return 1000;
        }
        let whole = done.min(total).saturating_mul(1000) / total;
        u16::try_from(whole).unwrap_or(1000)
    }

    /// The same, with `bytes` more written.
    pub fn wrote(self, bytes: u64) -> Progress {
        Progress {
            bytes_done: ByteSize(self.bytes_done.0.saturating_add(bytes)),
            ..self
        }
    }

    /// The same, with one more item finished.
    pub fn finished_one(self) -> Progress {
        Progress {
            items_done: self.items_done + 1,
            ..self
        }
    }
}
