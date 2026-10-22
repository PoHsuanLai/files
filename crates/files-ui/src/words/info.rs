//! What Get Info says of an item, as rows of label and words.

use std::time::SystemTime;

use files_core::{ByteSize, Entry, Info, ItemCount};
use files_model::ListFailure;

use super::{kind_words, size_words};

/// One row of Get Info: what it is and what it says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InfoRow {
    /// The row's name: `Kind`, `Size`, `Where`.
    pub label: &'static str,
    /// What it says.
    pub value: String,
}

impl InfoRow {
    pub(super) fn new(label: &'static str, value: String) -> InfoRow {
        InfoRow { label, value }
    }
}

/// `size` as Get Info says it: the words the list uses, then the exact bytes once they differ
/// from the words: `12 KB (12,345 bytes)`, `5 bytes`.
pub fn exact_size_words(size: ByteSize) -> String {
    let words = size_words(size);
    if size.0 < 1000 {
        words
    } else {
        format!("{words} ({} bytes)", grouped(size.0))
    }
}

/// `count` as `Zero items`, `1 item`, `12 items`.
pub fn items_words(count: ItemCount) -> String {
    match count.0 {
        0 => "Zero items".to_owned(),
        1 => "1 item".to_owned(),
        n => format!("{n} items"),
    }
}

/// The digits of `n` in groups of three, comma between.
fn grouped(n: u64) -> String {
    let digits = n.to_string();
    let first = digits.len() % 3;
    let mut out = String::with_capacity(digits.len() + digits.len() / 3);
    for (at, digit) in digits.chars().enumerate() {
        if at > 0 && at % 3 == first {
            out.push(',');
        }
        out.push(digit);
    }
    out
}

/// The rows Get Info shows for `info`, in order: Kind, Size (a file) or Contents (a folder),
/// Where, Created (when the filesystem says), Modified. `when` writes a date.
pub fn info_rows(info: &Info, when: impl Fn(Option<SystemTime>) -> String) -> Vec<InfoRow> {
    let kind = kind_words(&Entry::new(info.name.clone(), info.kind.clone()));
    let mut rows = vec![InfoRow::new("Kind", kind)];
    if let Some(size) = info.size() {
        rows.push(InfoRow::new("Size", exact_size_words(size)));
    }
    if let Some(count) = info.items {
        rows.push(InfoRow::new("Contents", items_words(count)));
    }
    rows.push(InfoRow::new(
        "Where",
        info.folder.as_path().display().to_string(),
    ));
    if info.created.is_some() {
        rows.push(InfoRow::new("Created", when(info.created)));
    }
    rows.push(InfoRow::new("Modified", when(info.modified)));
    rows
}

/// The rows for the name: `Name`, and `Extension` when it has one. Read-only for now.
pub fn name_rows(info: &Info) -> Vec<InfoRow> {
    let mut rows = vec![InfoRow::new("Name", info.name.display().into_owned())];
    if let Some(extension) = info.name.extension() {
        rows.push(InfoRow::new(
            "Extension",
            extension.to_string_lossy().into_owned(),
        ));
    }
    rows
}

/// What Get Info says when the facts of an item could not be read.
pub fn info_failure_words(failure: ListFailure) -> &'static str {
    match failure {
        ListFailure::NotFound => {
            "This item can\u{2019}t be found. It may have been moved or deleted."
        }
        ListFailure::NoPermission => "You don\u{2019}t have permission to see this item.",
        ListFailure::NotAFolder | ListFailure::Other => {
            "The details of this item can\u{2019}t be read."
        }
        // A failure files-model adds later reads as a general one until this names it.
        _ => "The details of this item can\u{2019}t be read.",
    }
}
