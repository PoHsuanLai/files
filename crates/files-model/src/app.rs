//! Opening with an app the person picked, and the sheet that lists every app.

use crate::browser::Browser;
use crate::effect::{Effect, OpenTarget};
use crate::open::{Opens, opens};

type Stepped = (Browser, Vec<Effect>);

/// An app the person picked to open files with, by the id of its desktop entry. The model never
/// looks inside: the caller that launches apps knows what it means.
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct AppPick(String);

impl AppPick {
    /// The app whose desktop entry has this id (`org.gnome.TextEditor.desktop`).
    pub fn new(id: impl Into<String>) -> AppPick {
        AppPick(id.into())
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// Whether the sheet that lists every installed app is up (Open With, Other).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Chooser {
    /// Not up.
    #[default]
    Closed,
    /// Up.
    Open,
}

impl Browser {
    /// Whether the sheet of every installed app is up.
    pub fn chooser(&self) -> Chooser {
        self.chooser
    }

    /// Open the selection with `app`: files and folders alike go to it, anything that opens
    /// nothing is left out. The sheet of apps, if up, goes away.
    pub(crate) fn open_with(self, app: AppPick) -> Stepped {
        let browser = Browser {
            chooser: Chooser::Closed,
            ..self
        };
        let targets: Vec<OpenTarget> = match browser.folder() {
            Some(folder) => browser
                .rows
                .iter()
                .filter(|entry| browser.selection.contains(&entry.name))
                .filter(|entry| opens(entry) != Opens::Nothing)
                .map(|entry| OpenTarget::new(folder.clone(), entry.name.clone()))
                .collect(),
            None => Vec::new(),
        };
        if targets.is_empty() {
            (browser, Vec::new())
        } else {
            (browser, vec![Effect::OpenFilesWith { targets, app }])
        }
    }

    /// Put up the sheet of every installed app, when something is selected in a folder.
    pub(crate) fn choose_app(self) -> Stepped {
        let chooser = match (self.folder(), self.selection.is_empty()) {
            (Some(_), false) => Chooser::Open,
            _ => Chooser::Closed,
        };
        (Browser { chooser, ..self }, Vec::new())
    }

    /// Put the sheet of apps away.
    pub(crate) fn close_chooser(self) -> Stepped {
        (
            Browser {
                chooser: Chooser::Closed,
                ..self
            },
            Vec::new(),
        )
    }

    /// The selected items of the folder shown, by where they are, in shown order.
    pub fn selected_targets(&self) -> Vec<OpenTarget> {
        match self.folder() {
            Some(folder) => self
                .rows
                .iter()
                .filter(|entry| self.selection.contains(&entry.name))
                .map(|entry| OpenTarget::new(folder.clone(), entry.name.clone()))
                .collect(),
            None => Vec::new(),
        }
    }
}
