//! Quick Look and Get Info: what the person has asked to see of an item.

use files_core::{Entry, FileName, FolderPath, Info, ItemRef};

use crate::browser::Browser;
use crate::effect::Effect;
use crate::listing::ListFailure;

type Stepped = (Browser, Vec<Effect>);

/// Whether Quick Look is up. It shows the row the keyboard is on, so it follows the arrow keys.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Looking {
    /// Not up.
    #[default]
    Closed,
    /// Up, over the row the keyboard is on.
    Open,
}

/// The Get Info panel: closed, waiting for the facts, showing them, or unable to.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
#[non_exhaustive]
pub enum InfoView {
    /// Not up.
    #[default]
    Closed,
    /// Up, waiting for the facts of this item.
    Loading(ItemRef),
    /// Up, showing the facts.
    Shown(Info),
    /// Up, and the facts of this item could not be read.
    Failed(ItemRef, ListFailure),
}

impl InfoView {
    /// The item the panel is about, when it is up.
    pub fn item(&self) -> Option<ItemRef> {
        match self {
            InfoView::Closed => None,
            InfoView::Loading(item) | InfoView::Failed(item, _) => Some(item.clone()),
            InfoView::Shown(info) => Some(info.item()),
        }
    }
}

impl Browser {
    /// The entry Quick Look is showing: the row the keyboard is on, while Quick Look is up.
    pub fn quick_look(&self) -> Option<&Entry> {
        match self.looking {
            Looking::Closed => None,
            Looking::Open => {
                let cursor = self.selection.cursor()?;
                self.rows.iter().find(|row| row.name == *cursor)
            }
        }
    }

    /// The item Quick Look is showing, by where it is.
    pub fn quick_item(&self) -> Option<ItemRef> {
        let entry = self.quick_look()?;
        Some(ItemRef::new(self.folder()?.clone(), entry.name.clone()))
    }

    /// The Get Info panel.
    pub fn info(&self) -> &InfoView {
        &self.info
    }

    /// Open Quick Look on the row the keyboard is on, or close it. It opens only in a folder on
    /// disk, over a row, and not while Get Info is up.
    pub(crate) fn toggle_quick_look(self) -> Stepped {
        let looking = match (self.looking, self.selection.cursor(), self.folder()) {
            (Looking::Open, _, _) => Looking::Closed,
            (Looking::Closed, Some(_), Some(_)) if self.info == InfoView::Closed => Looking::Open,
            (Looking::Closed, _, _) => Looking::Closed,
        };
        (Browser { looking, ..self }, Vec::new())
    }

    /// Put Quick Look away.
    pub(crate) fn close_quick_look(self) -> Stepped {
        (
            Browser {
                looking: Looking::Closed,
                ..self
            },
            Vec::new(),
        )
    }

    /// Open Get Info on the row the keyboard is on, or on the folder when nothing is.
    pub(crate) fn get_info(self) -> Stepped {
        let Some(folder) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        let item = match self.selection.cursor() {
            Some(name) => Some(ItemRef::new(folder.clone(), name.clone())),
            None => ItemRef::from_path(folder.as_path()).ok(),
        };
        match item {
            Some(item) => self.asking_info(item),
            None => (self, Vec::new()),
        }
    }

    /// Show `folder` with `names` selected, and open Get Info on the first of them (or on the
    /// folder itself when none is named).
    pub(crate) fn properties(self, folder: FolderPath, names: Vec<FileName>) -> Stepped {
        let item = match names.first() {
            Some(name) => Some(ItemRef::new(folder.clone(), name.clone())),
            None => ItemRef::from_path(folder.as_path()).ok(),
        };
        let (browser, mut effects) = self.revealing(folder, names);
        match item {
            Some(item) => {
                let (browser, more) = browser.asking_info(item);
                effects.extend(more);
                (browser, effects)
            }
            None => (browser, effects),
        }
    }

    /// Get Info up on `item`, waiting for its facts. Quick Look goes away.
    fn asking_info(self, item: ItemRef) -> Stepped {
        let effects = vec![Effect::ReadInfo(item.clone())];
        let browser = Browser {
            looking: Looking::Closed,
            info: InfoView::Loading(item),
            ..self
        };
        (browser, effects)
    }

    /// The answer to `Effect::ReadInfo`. One for an item the panel is no longer about is ignored.
    pub(crate) fn info_read(self, item: ItemRef, result: Result<Info, ListFailure>) -> Stepped {
        if self.info.item().as_ref() != Some(&item) {
            return (self, Vec::new());
        }
        let info = match result {
            Ok(info) => InfoView::Shown(info),
            Err(failure) => InfoView::Failed(item, failure),
        };
        (Browser { info, ..self }, Vec::new())
    }

    /// Put Get Info away.
    pub(crate) fn close_info(self) -> Stepped {
        (
            Browser {
                info: InfoView::Closed,
                ..self
            },
            Vec::new(),
        )
    }
}
