//! What is said while files change: the toast after a move to the Trash, the progress sheet and
//! the questions, in Finder's words. Pure.

use files_core::{Conflict, ConflictKind, OpKind, Summary};

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
