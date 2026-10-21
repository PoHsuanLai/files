//! Line helpers shared by the parsers.

/// The lines of `text` that carry data: trimmed, not empty, not comments.
pub(crate) fn data_lines(text: &str) -> impl Iterator<Item = &str> {
    text.lines()
        .map(str::trim)
        .filter(|line| !line.is_empty() && !line.starts_with('#'))
}

/// One `key=value` line and the `[group]` it sits in (empty before the first group).
pub(crate) struct KeyLine<'a> {
    pub group: &'a str,
    pub key: &'a str,
    pub value: &'a str,
}

/// Every `key=value` line of a desktop-entry-like file, in order.
pub(crate) fn key_lines(text: &str) -> Vec<KeyLine<'_>> {
    let mut group = "";
    let mut out = Vec::new();
    for line in data_lines(text) {
        if let Some(name) = line.strip_prefix('[').and_then(|rest| rest.strip_suffix(']')) {
            group = name;
        } else if let Some((key, value)) = line.split_once('=') {
            out.push(KeyLine { group, key: key.trim(), value: value.trim() });
        }
    }
    out
}
