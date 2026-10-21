//! How two entries order under a [`Sort`].

use std::cmp::Ordering;
use std::os::unix::ffi::OsStrExt;

use files_core::{ByteSize, Entry, EntryKind, LinkTo, Sort, SortKey, SortOrder};

use crate::open::{Opens, opens};

/// How `a` orders against `b` under `sort`. The order is total: entries that tie on the key tie
/// on the name, and names that tie there are compared by their raw bytes.
///
/// Name sorting is natural and case-insensitive (`file2` before `file10`) and does not put
/// folders first; Kind sorting does. Descending reverses the whole order.
pub fn compare(a: &Entry, b: &Entry, sort: Sort) -> Ordering {
    let ordered = match sort.key {
        SortKey::Name => by_name(a, b),
        SortKey::Modified => a.modified.cmp(&b.modified).then_with(|| by_name(a, b)),
        SortKey::Size => size(a).cmp(&size(b)).then_with(|| by_name(a, b)),
        SortKey::Kind => kind_key(a).cmp(&kind_key(b)).then_with(|| by_name(a, b)),
        _ => by_name(a, b),
    };
    match sort.order {
        SortOrder::Ascending => ordered,
        SortOrder::Descending => ordered.reverse(),
    }
}

fn by_name(a: &Entry, b: &Entry) -> Ordering {
    let (x, y) = (a.name.as_os_str().as_bytes(), b.name.as_os_str().as_bytes());
    natural(x, y).then_with(|| x.cmp(y))
}

/// Folders count as no size.
fn size(entry: &Entry) -> ByteSize {
    match &entry.kind {
        EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) => *size,
        EntryKind::Folder | EntryKind::Link(_) | EntryKind::Special(_) => ByteSize(0),
        // A kind files-core adds later sorts as no size until this names it.
        _ => ByteSize(0),
    }
}

/// Folders first, then by type, then by extension.
fn kind_key(entry: &Entry) -> (u8, Option<&str>, Option<Vec<u8>>) {
    let rank = match opens(entry) {
        Opens::Folder => 0,
        Opens::File | Opens::Nothing => 1,
    };
    let extension = entry
        .name
        .extension()
        .map(|e| e.as_bytes().to_ascii_lowercase());
    (rank, entry.mime.as_ref().map(|m| m.as_str()), extension)
}

/// Letters compare ignoring ASCII case, runs of digits compare as numbers.
fn natural(a: &[u8], b: &[u8]) -> Ordering {
    let (mut i, mut j) = (0, 0);
    while let (Some(&x), Some(&y)) = (a.get(i), b.get(j)) {
        let step = if x.is_ascii_digit() && y.is_ascii_digit() {
            let (run_a, next_a) = digits(a, i);
            let (run_b, next_b) = digits(b, j);
            (i, j) = (next_a, next_b);
            number(run_a).cmp(&number(run_b))
        } else {
            (i, j) = (i + 1, j + 1);
            x.to_ascii_lowercase().cmp(&y.to_ascii_lowercase())
        };
        if step != Ordering::Equal {
            return step;
        }
    }
    (a.len() - i).cmp(&(b.len() - j))
}

/// The run of digits starting at `from`, and where it ends.
fn digits(text: &[u8], from: usize) -> (&[u8], usize) {
    let rest = text.get(from..).unwrap_or_default();
    let len = rest.iter().take_while(|b| b.is_ascii_digit()).count();
    (rest.get(..len).unwrap_or_default(), from + len)
}

/// A digit run without its leading zeros, as a pair that orders like the number: longer is bigger.
fn number(run: &[u8]) -> (usize, &[u8]) {
    let start = run.iter().take_while(|b| **b == b'0').count();
    let trimmed = run.get(start..).unwrap_or_default();
    (trimmed.len(), trimmed)
}
