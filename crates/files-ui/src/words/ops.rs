//! What is said while files change: the toast after a move to the Trash, the progress sheet and
//! the questions, in Finder's words. Pure.

use files_core::{Conflict, ConflictKind, OpKind, Summary};
use files_model::ExtensionChange;

/// `“name”` in typographic quotes.
fn quoted(summary: &Summary) -> String {
    let name = summary.first.as_ref().map(|n| n.display().into_owned());
    format!("\u{201c}{}\u{201d}", name.unwrap_or_default())
}

/// The toast after a move to the Trash: `Moved “x” to the Trash`, or the count for several.
pub fn announce_words(summary: &Summary) -> String {
    match summary.count {
        1 => format!("Moved {} to the Trash", quoted(summary)),
        count => format!("Moved {count} items to the Trash"),
    }
}

/// What the progress sheet says is going on: `Copying “x”`, `Moving 3 items to the Trash`.
pub fn busy_words(summary: &Summary) -> String {
    let (doing, after) = match summary.kind {
        OpKind::Copy => ("Copying", ""),
        OpKind::Move => ("Moving", ""),
        OpKind::NewFolder => ("Making a folder", ""),
        OpKind::Rename => ("Renaming", ""),
        OpKind::Trash => ("Moving", " to the Trash"),
        OpKind::PutBack => ("Putting back", ""),
        OpKind::Erase => ("Erasing", ""),
        // A kind files-core adds later is just work until this names it.
        _ => ("Working on", ""),
    };
    match summary.count {
        0 => format!("{doing}{after}"),
        1 => format!("{doing} {}{after}", quoted(summary)),
        count => format!("{doing} {count} items{after}"),
    }
}

/// The question about a taken name: its title, and the message under it.
pub fn conflict_words(conflict: &Conflict) -> (String, &'static str) {
    let title = format!(
        "An item named \u{201c}{}\u{201d} already exists in this location.",
        conflict.item
    );
    let message = match conflict.kind {
        ConflictKind::Copy => "Do you want to replace it with the one you're copying?",
        ConflictKind::Move => "Do you want to replace it with the one you're moving?",
        // A kind files-core adds later asks about moving until this names it.
        _ => "Do you want to replace it with the one you're moving?",
    };
    (title, message)
}

/// The title of the question before erasing `count` items of the Trash for good, the first being
/// `first`; `None` for the whole Trash.
pub fn erase_words(count: Option<(usize, String)>) -> String {
    match count {
        None => "Are you sure you want to permanently erase the items in the Trash?".to_owned(),
        Some((1, name)) => {
            format!("Are you sure you want to permanently erase \u{201c}{name}\u{201d}?")
        }
        Some((count, _)) => {
            format!("Are you sure you want to permanently erase these {count} items?")
        }
    }
}

/// The message under any question about erasing.
pub const CANNOT_UNDO: &str = "You can't undo this action.";

/// What the question before changing a file's extension says.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ExtensionWords {
    /// The question.
    pub title: String,
    /// The line under it.
    pub message: &'static str,
    /// The button that goes ahead with the new name.
    pub go_ahead: String,
    /// The button that keeps the old extension.
    pub keep: String,
}

/// The question before a rename that changes, adds or takes away a file's extension.
pub fn extension_words(change: &ExtensionChange) -> ExtensionWords {
    let quoted = |extension: &str| format!("\u{201c}.{extension}\u{201d}");
    let message = "If you make this change, your document may open in a different application.";
    match (change.old_extension(), change.new_extension()) {
        (Some(old), Some(new)) => ExtensionWords {
            title: format!(
                "Are you sure you want to change the extension from {} to {}?",
                quoted(&old),
                quoted(&new)
            ),
            message,
            go_ahead: format!("Use .{new}"),
            keep: format!("Keep .{old}"),
        },
        (None, Some(new)) => ExtensionWords {
            title: format!(
                "Are you sure you want to add the extension {} to the end of the name?",
                quoted(&new)
            ),
            message,
            go_ahead: format!("Add .{new}"),
            keep: "Don't Add".to_owned(),
        },
        (Some(old), None) => ExtensionWords {
            title: format!(
                "Are you sure you want to remove the extension {}?",
                quoted(&old)
            ),
            message,
            go_ahead: "Remove".to_owned(),
            keep: format!("Keep .{old}"),
        },
        // Neither name has one: nothing to ask, but the words stay plain.
        (None, None) => ExtensionWords {
            title: "Are you sure you want to rename this item?".to_owned(),
            message,
            go_ahead: "Rename".to_owned(),
            keep: "Cancel".to_owned(),
        },
    }
}
