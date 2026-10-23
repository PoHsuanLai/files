//! Cloud drives in the browser: the drives as told, where each visible row stands, opening a file
//! that is not downloaded yet, and the asks of the context menu (Download Now, Remove Download,
//! Keep Downloaded).

use files_core::{
    CloudDrive, CloudItem, CloudKind, CloudState, DriveState, FileName, FolderPath, ItemRef,
    Keeping, Refused, SavedCopies,
};

use crate::browser::Browser;
use crate::effect::{Effect, OpenTarget};
use crate::history::Availability;

type Stepped = (Browser, Vec<Effect>);

/// The most rows whose cloud state is asked for at once (the model does not know the viewport).
pub const ROWS_ASKED: usize = 1000;

/// What opening a file says while its drive is offline and the file is not downloaded.
pub const OFFLINE_WORDS: &str = "This file will download when you\u{2019}re connected.";

/// What opening an online document says when it has no address.
pub const NO_ADDRESS_WORDS: &str = "This document can\u{2019}t be opened online right now.";

/// What a download that ended in an error says when the service gave no words.
pub const DOWNLOAD_FAILED_WORDS: &str = "The file couldn\u{2019}t be downloaded.";

/// How much of the selection is kept downloaded.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum KeptMark {
    /// All of it.
    All,
    /// Some of it.
    Partly,
    /// None of it.
    Nothing,
}

/// What the context menu offers for the selection in a cloud drive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[non_exhaustive]
pub struct CloudMenu {
    /// Download Now.
    pub download: Availability,
    /// Remove Download.
    pub remove: Availability,
    /// Keep Downloaded.
    pub keep: Availability,
    /// Whether Keep Downloaded is checked.
    pub kept: KeptMark,
    /// Save a Copy: available when the selection holds an online document.
    pub copy: Availability,
}

impl Browser {
    /// The cloud drives the sidebar lists, in the order the caller gave them.
    pub fn clouds(&self) -> &[CloudDrive] {
        &self.clouds
    }

    /// The drive the folder shown is in (or is).
    pub fn drive_here(&self) -> Option<&CloudDrive> {
        let here = self.folder()?;
        self.clouds.iter().find(|drive| drive.holds(here))
    }

    /// The drive whose own folder is `folder`.
    pub fn drive_at(&self, folder: &FolderPath) -> Option<&CloudDrive> {
        self.clouds.iter().find(|drive| drive.folder == *folder)
    }

    /// Where the row called `name` stands, when it has been read.
    pub fn cloud_of(&self, name: &FileName) -> Option<&CloudItem> {
        self.cloud.get(name)
    }

    /// What the context menu offers for the selection; `None` outside a cloud drive or with
    /// nothing selected.
    pub fn cloud_menu(&self) -> Option<CloudMenu> {
        self.drive_here()?;
        let real = self.real_selection();
        if self.selected_rows().is_empty() {
            return None;
        }
        let when = |test: fn(Option<&CloudItem>) -> bool| {
            if self
                .real_selection()
                .iter()
                .any(|(name, _)| test(self.cloud.get(name)))
            {
                Availability::Available
            } else {
                Availability::Unavailable
            }
        };
        let kept = |item: &Option<&CloudItem>| item.is_some_and(|i| i.keeping == Keeping::Kept);
        let items: Vec<Option<&CloudItem>> =
            real.iter().map(|(name, _)| self.cloud.get(name)).collect();
        let kept_mark = if items.is_empty() || !items.iter().any(kept) {
            KeptMark::Nothing
        } else if items.iter().all(kept) {
            KeptMark::All
        } else {
            KeptMark::Partly
        };
        Some(CloudMenu {
            download: when(wants_download),
            remove: when(can_remove),
            keep: when(|_| true),
            kept: kept_mark,
            copy: if self.selected_online().is_empty() {
                Availability::Unavailable
            } else {
                Availability::Available
            },
        })
    }

    /// The selected rows that are not online documents (which are never downloaded, kept or
    /// removed), by name and item.
    fn real_selection(&self) -> Vec<(FileName, ItemRef)> {
        self.selected_rows()
            .into_iter()
            .filter(|(row, _)| {
                self.cloud
                    .get(&row.name)
                    .is_none_or(|item| item.kind != CloudKind::Online)
            })
            .map(|(row, item)| (row.name.clone(), item))
            .collect()
    }

    /// What to ask for on a fresh listing of a cloud folder: where its rows stand.
    pub(crate) fn cloud_ask(&self) -> Vec<Effect> {
        if self.drive_here().is_none() {
            return Vec::new();
        }
        let items: Vec<ItemRef> = self
            .rows
            .iter()
            .filter_map(|row| self.item_of(row))
            .take(ROWS_ASKED)
            .collect();
        if items.is_empty() {
            Vec::new()
        } else {
            vec![Effect::ReadCloud(items)]
        }
    }

    /// The drives changed. Arriving in a drive's folder (it was not known yet) asks where the
    /// rows stand.
    pub(crate) fn clouds_changed(self, clouds: Vec<CloudDrive>) -> Stepped {
        let known = self.drive_here().is_some();
        let browser = Browser { clouds, ..self };
        let arrived = !known && browser.drive_here().is_some();
        let effects = if arrived {
            browser.cloud_ask()
        } else {
            Vec::new()
        };
        (browser, effects)
    }

    /// News of where items stand: those in the folder shown are kept, and a file waiting to be
    /// opened is opened once it is downloaded.
    pub(crate) fn cloud_read(mut self, items: Vec<(ItemRef, CloudItem)>) -> Stepped {
        let here = self.folder().cloned();
        for (item, details) in items {
            if Some(&item.folder) == here.as_ref() {
                self.cloud.insert(item.name, details);
            }
        }
        self.opened_when_ready()
    }

    /// The files waiting for their download that are downloaded now are opened; one that failed
    /// is told; one that went back to the cloud stops waiting.
    fn opened_when_ready(mut self) -> Stepped {
        let Some(folder) = self.folder().cloned() else {
            return (self, Vec::new());
        };
        let waiting = std::mem::take(&mut self.opening);
        let mut effects = Vec::new();
        let mut ready = Vec::new();
        for name in waiting {
            match self
                .cloud
                .get(&name)
                .map(|item| (item.state, item.words.clone()))
            {
                Some((CloudState::Downloaded, _)) => {
                    ready.push(OpenTarget::new(folder.clone(), name));
                }
                Some((CloudState::Error, words)) => {
                    let said = words.map_or_else(
                        || DOWNLOAD_FAILED_WORDS.to_owned(),
                        |words| words.to_string(),
                    );
                    effects.push(Effect::Say(said));
                }
                Some((CloudState::Cloud, _)) => {}
                Some(_) | None => self.opening.push(name),
            }
        }
        if !ready.is_empty() {
            effects.insert(0, Effect::OpenFiles(ready));
        }
        (self, effects)
    }

    /// Open `files` of a folder: in a cloud drive a file that is not downloaded is downloaded
    /// first and opened when it is, and an online document opens in the browser.
    pub(crate) fn open_files(mut self, files: Vec<OpenTarget>) -> Stepped {
        let Some(drive) = self.drive_here() else {
            return (self, vec![Effect::OpenFiles(files)]);
        };
        let offline = drive.state == DriveState::Offline;
        let mut now = Vec::new();
        let mut download = Vec::new();
        let mut effects = Vec::new();
        for target in files {
            let found = self.cloud.get(&target.name).cloned();
            match found {
                Some(item) if item.kind == CloudKind::Online => effects.push(match item.url {
                    Some(url) => Effect::OpenUrl(url),
                    None => Effect::Say(NO_ADDRESS_WORDS.to_owned()),
                }),
                Some(item) if item.state == CloudState::Cloud && offline => {
                    effects.push(Effect::Say(OFFLINE_WORDS.to_owned()));
                }
                Some(item) if item.state == CloudState::Cloud => {
                    download.push(ItemRef::new(target.folder, target.name.clone()));
                    self.opening.push(target.name);
                }
                Some(item) if item.state == CloudState::Downloading => {
                    if !self.opening.contains(&target.name) {
                        self.opening.push(target.name);
                    }
                }
                Some(_) | None => now.push(target),
            }
        }
        let mut all = Vec::new();
        if !now.is_empty() {
            all.push(Effect::OpenFiles(now));
        }
        if !download.is_empty() {
            all.push(Effect::CloudDownload(download));
        }
        all.extend(effects);
        (self, all)
    }

    /// Download Now for the selection.
    pub(crate) fn download_now(self) -> Stepped {
        let items = self.selected_where(wants_download);
        self.asking_cloud(items, Effect::CloudDownload)
    }

    /// Remove Download for the selection.
    pub(crate) fn remove_download(self) -> Stepped {
        let items = self.selected_where(can_remove);
        self.asking_cloud(items, Effect::CloudRemove)
    }

    /// Keep Downloaded for the selection: off when all of it is kept, else on.
    pub(crate) fn keep_downloaded(self) -> Stepped {
        let items = self.selected_where(|_| true);
        let all_kept = {
            let real = self.real_selection();
            !real.is_empty()
                && real.iter().all(|(name, _)| {
                    self.cloud
                        .get(name)
                        .is_some_and(|i| i.keeping == Keeping::Kept)
                })
        };
        let keeping = if all_kept {
            Keeping::NotKept
        } else {
            Keeping::Kept
        };
        self.asking_cloud(items, move |items| Effect::CloudKeep(items, keeping))
    }

    fn selected_where(&self, wanted: impl Fn(Option<&CloudItem>) -> bool) -> Vec<ItemRef> {
        if self.drive_here().is_none() {
            return Vec::new();
        }
        self.real_selection()
            .into_iter()
            .filter(|(name, _)| wanted(self.cloud.get(name)))
            .map(|(_, item)| item)
            .collect()
    }

    fn asking_cloud(
        self,
        items: Vec<ItemRef>,
        ask: impl FnOnce(Vec<ItemRef>) -> Effect,
    ) -> Stepped {
        if items.is_empty() {
            (self, Vec::new())
        } else {
            (self, vec![ask(items)])
        }
    }

    /// The selected online documents, by item.
    fn selected_online(&self) -> Vec<ItemRef> {
        if self.drive_here().is_none() {
            return Vec::new();
        }
        self.selected_rows()
            .into_iter()
            .filter(|(row, _)| {
                self.cloud
                    .get(&row.name)
                    .is_some_and(|item| item.kind == CloudKind::Online)
            })
            .map(|(_, item)| item)
            .collect()
    }

    /// Save a Copy for the online documents in the selection.
    pub(crate) fn save_copy(self) -> Stepped {
        let items = self.selected_online();
        self.asking_cloud(items, Effect::CloudSaveCopy)
    }

    /// The answer to Save a Copy: the copies made in the folder shown are selected as soon as the
    /// listing holds them (the folder is read again), their states are asked for, and each
    /// refusal is told.
    pub(crate) fn copies_saved(self, copies: SavedCopies) -> Stepped {
        let here = self.folder().cloned();
        let made: Vec<ItemRef> = copies
            .saved
            .into_iter()
            .map(|saved| saved.copy)
            .filter(|copy| Some(&copy.folder) == here.as_ref())
            .collect();
        let names: Vec<FileName> = made.iter().map(|copy| copy.name.clone()).collect();
        if names.is_empty() {
            return self.cloud_answered(copies.refused);
        }
        // The folder is read again, so the copies are listed whether or not a change was seen.
        let (browser, mut effects) = Browser {
            reveal: names,
            ..self
        }
        .reload();
        effects.push(Effect::ReadCloud(made));
        let (browser, said) = browser.cloud_answered(copies.refused);
        effects.extend(said);
        (browser, effects)
    }

    /// The answer to an ask of the drives: each reason a path was refused is told once, and a
    /// file refused stops waiting to be opened.
    pub(crate) fn cloud_answered(mut self, refused: Vec<Refused>) -> Stepped {
        let mut said: Vec<String> = Vec::new();
        for Refused { item, words } in refused {
            self.opening.retain(|name| *name != item.name);
            let text = words.to_string();
            if !said.contains(&text) {
                said.push(text);
            }
        }
        let effects = said.into_iter().map(Effect::Say).collect();
        (self, effects)
    }
}

/// Whether Download Now has something to do for an item: a folder, a file in the cloud or one
/// that failed, or one not read yet.
fn wants_download(item: Option<&CloudItem>) -> bool {
    item.is_none_or(|item| {
        item.kind == CloudKind::Folder
            || item.state == CloudState::Cloud
            || item.state == CloudState::Error
    })
}

/// Whether Remove Download has something to do for an item: a folder, or a downloaded file.
fn can_remove(item: Option<&CloudItem>) -> bool {
    item.is_some_and(|item| item.kind == CloudKind::Folder || item.state == CloudState::Downloaded)
}
