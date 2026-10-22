//! Showing named entries of a folder: "Show in Folder" from another program.

use files_core::{Entry, FileName, FolderPath, Location};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::listing::Listing;
use crate::selection::Selection;

type Stepped = (Browser, Vec<Effect>);

impl Browser {
    /// Go to `folder` (a visit like any other; the folder already shown is read again, since the
    /// file may have just been made) and select `names` in it. What is not listed yet is selected
    /// when it arrives.
    pub(crate) fn revealing(self, folder: FolderPath, names: Vec<FileName>) -> Stepped {
        let (browser, effects) = self.go(Location::Folder(folder));
        let browser = Browser {
            reveal: names,
            ..browser
        };
        (browser.refreshed(), effects)
    }
}

/// `selection`, or the entries of `rows` that `asked` names once the listing is in; the first one
/// asked for that is there is the cursor. When none is there, the selection stays as it is.
pub(crate) fn applied(
    listing: &Listing,
    rows: &[Entry],
    asked: &[FileName],
    selection: Selection,
) -> Selection {
    match listing {
        Listing::Loaded(_) => {}
        Listing::Loading | Listing::Failed(_) => return selection,
    }
    let found: Vec<FileName> = asked
        .iter()
        .filter(|name| rows.iter().any(|row| row.name == **name))
        .cloned()
        .collect();
    if found.is_empty() {
        selection
    } else {
        Selection::of(found)
    }
}
