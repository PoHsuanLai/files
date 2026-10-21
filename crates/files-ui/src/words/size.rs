//! Sizes as Finder says them: decimal units, whole kilobytes, one decimal from megabytes up.

use files_core::{ByteSize, Entry, EntryKind, LinkTo};

use super::NONE;

/// Units above bytes, each a thousand of the one before.
const UNITS: [&str; 5] = ["KB", "MB", "GB", "TB", "PB"];

/// `size` in words: `Zero bytes`, `1 byte`, `512 bytes`, `4 KB`, `1.5 MB`, `12 GB`.
///
/// Kilobytes are whole; megabytes and up keep one decimal, dropped when it is zero. A size that
/// rounds up to a thousand of its unit is shown in the next one (`999999` is `1 MB`).
pub fn size_words(size: ByteSize) -> String {
    match size.0 {
        0 => "Zero bytes".to_owned(),
        1 => "1 byte".to_owned(),
        bytes @ 2..=999 => format!("{bytes} bytes"),
        bytes => scaled(bytes),
    }
}

/// What the Size column says of `entry`: a file's size, or a dash for anything with none (a folder,
/// a link that leads nowhere, a pipe).
pub fn entry_size_words(entry: &Entry) -> String {
    match &entry.kind {
        EntryKind::File(size) | EntryKind::Link(LinkTo::File(size)) => size_words(*size),
        EntryKind::Folder | EntryKind::Link(_) | EntryKind::Special(_) => NONE.to_owned(),
        // A kind files-core adds later has no size until this names it.
        _ => NONE.to_owned(),
    }
}

/// `bytes` (a thousand or more) in kilobytes or a bigger unit.
fn scaled(bytes: u64) -> String {
    let mut value = bytes as f64 / 1000.0;
    let mut unit = 0;
    loop {
        let shown = rounded(unit, value);
        if shown < 1000.0 || unit + 1 == UNITS.len() {
            return written(unit, shown);
        }
        value /= 1000.0;
        unit += 1;
    }
}

/// `value` of `UNITS[unit]` rounded as it is shown: whole kilobytes, tenths above.
fn rounded(unit: usize, value: f64) -> f64 {
    if unit == 0 {
        value.round()
    } else {
        (value * 10.0).round() / 10.0
    }
}

/// `shown` of `UNITS[unit]` as text.
fn written(unit: usize, shown: f64) -> String {
    let name = UNITS.get(unit).copied().unwrap_or("PB");
    if shown.fract() == 0.0 {
        format!("{shown:.0} {name}")
    } else {
        format!("{shown:.1} {name}")
    }
}
