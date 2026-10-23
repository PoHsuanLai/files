//! Cloud drives a test controls.

use std::collections::BTreeMap;
use std::sync::{Mutex, MutexGuard, PoisonError};

use files_core::{
    CloudDrive, CloudItem, CloudKind, CloudNews, CloudState, CloudWords, DriveId, DriveState,
    ItemRef, Keeping, Refused, Saved, SavedCopies, Share,
};

use super::{CloudError, CloudGuard, Clouds, News};

/// What was asked of a [`FakeClouds`].
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum CloudCall {
    /// Download Now.
    Downloaded(Vec<ItemRef>),
    /// Remove Download.
    Removed(Vec<ItemRef>),
    /// Keep Downloaded, on or off.
    Kept(Vec<ItemRef>, Keeping),
    /// Save a Copy.
    Copied(Vec<ItemRef>),
}

/// What a download asked of an offline drive says.
pub const OFFLINE_WORDS: &str = "This file will download when you\u{2019}re connected.";

/// What a document with no file format (a Form) is refused with.
pub const NO_FILE_FORMAT_WORDS: &str = "This document can\u{2019}t be saved as a file.";

/// What anything but an online document is refused with.
const NOT_ONLINE_WORDS: &str = "Only online documents can be saved as a copy.";

/// The file type a document of the extension `ext` is saved as.
fn export_of(ext: &str) -> Option<&'static str> {
    match ext {
        "gdoc" => Some("docx"),
        "gsheet" => Some("xlsx"),
        "gslides" => Some("pptx"),
        "gdraw" => Some("png"),
        _ => None,
    }
}

#[derive(Default)]
struct State {
    drives: Vec<CloudDrive>,
    items: BTreeMap<ItemRef, CloudItem>,
    calls: Vec<CloudCall>,
    away: bool,
    news: Option<News>,
}

/// Drives kept in memory. A download starts at nothing and moves only when the test says
/// ([`FakeClouds::advance`], [`FakeClouds::finish`]), so every step can be looked at.
#[derive(Default)]
pub struct FakeClouds {
    state: Mutex<State>,
}

impl FakeClouds {
    /// A computer with these drives and no items.
    pub fn with(drives: Vec<CloudDrive>) -> FakeClouds {
        let fake = FakeClouds::default();
        fake.lock().drives = drives;
        fake
    }

    /// The same computer knowing `item` as `details`.
    pub fn holding(self, item: ItemRef, details: CloudItem) -> FakeClouds {
        self.lock().items.insert(item, details);
        self
    }

    /// The drives are now these, and anyone watching is told.
    pub fn set_drives(&self, drives: Vec<CloudDrive>) {
        let mut state = self.lock();
        state.drives = drives;
        tell(&state, CloudNews::Drives);
    }

    /// The service goes away (`true`) or comes back (`false`); anyone watching is told.
    pub fn set_away(&self, away: bool) {
        let mut state = self.lock();
        state.away = away;
        tell(&state, CloudNews::Drives);
    }

    /// `item` is now `details`, and anyone watching is told.
    pub fn set_item(&self, item: ItemRef, details: CloudItem) {
        let mut state = self.lock();
        state.items.insert(item.clone(), details.clone());
        let drive = drive_of(&state.drives, &item);
        if let Some(drive) = drive {
            tell(&state, CloudNews::Items(drive, vec![(item, details)]));
        }
    }

    /// A moving transfer of `item` is `share` of the way.
    pub fn advance(&self, item: &ItemRef, share: Share) {
        let found = self.lock().items.get(item).cloned();
        if let Some(details) = found {
            self.set_item(item.clone(), details.moving(share));
        }
    }

    /// The transfer of `item` ended: it is downloaded.
    pub fn finish(&self, item: &ItemRef) {
        let found = self.lock().items.get(item).cloned();
        if let Some(details) = found {
            let mut done = details;
            done.state = CloudState::Downloaded;
            done.share = None;
            self.set_item(item.clone(), done);
        }
    }

    /// Everything asked so far, in order.
    pub fn calls(&self) -> Vec<CloudCall> {
        self.lock().calls.clone()
    }

    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

impl std::fmt::Debug for FakeClouds {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str("FakeClouds")
    }
}

fn tell(state: &State, news: CloudNews) {
    if let Some(news_to) = &state.news {
        news_to(news);
    }
}

fn drive_of(drives: &[CloudDrive], item: &ItemRef) -> Option<DriveId> {
    drives
        .iter()
        .find(|drive| drive.holds(&item.folder))
        .map(|drive| drive.id.clone())
}

fn offline(state: &State, item: &ItemRef) -> bool {
    state
        .drives
        .iter()
        .any(|drive| drive.holds(&item.folder) && drive.state == DriveState::Offline)
}

impl FakeClouds {
    /// Runs `change` on every item of `items` the fake knows, telling what changed; the items
    /// `change` refuses come back with its words.
    fn each(
        &self,
        items: &[ItemRef],
        change: impl Fn(&State, &ItemRef, &CloudItem) -> Result<CloudItem, CloudWords>,
    ) -> Vec<Refused> {
        let mut state = self.lock();
        let mut refused = Vec::new();
        for item in items {
            let Some(found) = state.items.get(item).cloned() else {
                continue;
            };
            match change(&state, item, &found) {
                Ok(next) => {
                    state.items.insert(item.clone(), next.clone());
                    if let Some(drive) = drive_of(&state.drives, item) {
                        tell(&state, CloudNews::Items(drive, vec![(item.clone(), next)]));
                    }
                }
                Err(words) => refused.push(Refused {
                    item: item.clone(),
                    words,
                }),
            }
        }
        refused
    }
}

fn started(found: &CloudItem) -> CloudItem {
    match found.state {
        CloudState::Cloud | CloudState::Error => {
            let mut next = found.clone();
            next.state = CloudState::Downloading;
            next.share = Some(Share::default());
            next.words = None;
            next
        }
        CloudState::Downloading
        | CloudState::Downloaded
        | CloudState::Uploading
        | CloudState::Waiting
        | CloudState::Conflict
        | _ => found.clone(),
    }
}

impl Clouds for FakeClouds {
    fn drives(&self) -> Result<Vec<CloudDrive>, CloudError> {
        let state = self.lock();
        if state.away {
            return Err(CloudError::Unavailable);
        }
        Ok(state.drives.clone())
    }

    fn items(&self, items: &[ItemRef]) -> Result<Vec<(ItemRef, CloudItem)>, CloudError> {
        let state = self.lock();
        if state.away {
            return Err(CloudError::Unavailable);
        }
        Ok(items
            .iter()
            .filter_map(|item| Some((item.clone(), state.items.get(item)?.clone())))
            .collect())
    }

    fn download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, CloudError> {
        self.check()?;
        self.lock()
            .calls
            .push(CloudCall::Downloaded(items.to_vec()));
        Ok(self.each(items, |state, item, found| {
            let starts = started(found) != *found;
            if starts && offline(state, item) {
                Err(CloudWords::new(OFFLINE_WORDS))
            } else {
                Ok(started(found))
            }
        }))
    }

    fn remove_download(&self, items: &[ItemRef]) -> Result<Vec<Refused>, CloudError> {
        self.check()?;
        self.lock().calls.push(CloudCall::Removed(items.to_vec()));
        Ok(self.each(items, |_, _, found| match found.state {
            CloudState::Waiting | CloudState::Uploading => {
                Err(CloudWords::new("Waiting to upload."))
            }
            CloudState::Conflict => Err(CloudWords::new(
                "This file has two versions. Choose one first.",
            )),
            CloudState::Cloud
            | CloudState::Downloading
            | CloudState::Downloaded
            | CloudState::Error
            | _ => {
                let mut next = found.clone();
                next.state = CloudState::Cloud;
                next.keeping = Keeping::NotKept;
                next.share = None;
                Ok(next)
            }
        }))
    }

    fn save_copy(&self, items: &[ItemRef]) -> Result<SavedCopies, CloudError> {
        self.check()?;
        self.lock().calls.push(CloudCall::Copied(items.to_vec()));
        let mut answer = SavedCopies::default();
        for document in items {
            let found = self.lock().items.get(document).cloned();
            let refuse = |words: &str| Refused {
                item: document.clone(),
                words: CloudWords::new(words),
            };
            match found {
                Some(found) if found.kind == CloudKind::Online => {
                    let ext = document
                        .name
                        .extension()
                        .map(|e| e.to_string_lossy().into_owned());
                    let Some(export) = ext.as_deref().and_then(export_of) else {
                        answer.refused.push(refuse(NO_FILE_FORMAT_WORDS));
                        continue;
                    };
                    let stem = document.name.stem().to_string_lossy().into_owned();
                    let copy = self.free_name(document, &stem, export);
                    match copy {
                        Some(copy) => {
                            let file = CloudItem::new(CloudKind::File, CloudState::Downloaded);
                            self.set_item(copy.clone(), file);
                            answer.saved.push(Saved {
                                document: document.clone(),
                                copy,
                            });
                        }
                        None => answer.refused.push(refuse(NO_FILE_FORMAT_WORDS)),
                    }
                }
                Some(_) | None => answer.refused.push(refuse(NOT_ONLINE_WORDS)),
            }
        }
        Ok(answer)
    }

    fn keep(&self, items: &[ItemRef], keeping: Keeping) -> Result<Vec<Refused>, CloudError> {
        self.check()?;
        self.lock()
            .calls
            .push(CloudCall::Kept(items.to_vec(), keeping));
        Ok(self.each(items, |_, _, found| {
            let mut next = found.clone();
            next.keeping = keeping;
            match keeping {
                Keeping::Kept => Ok(started(&next)),
                Keeping::NotKept => Ok(next),
            }
        }))
    }

    fn watch(&self, news: News) -> Result<CloudGuard, CloudError> {
        self.check()?;
        self.lock().news = Some(news);
        Ok(CloudGuard::new(()))
    }
}

impl FakeClouds {
    /// "Plan.docx", else "Plan 2.docx", "Plan 3.docx"... the first the fake does not hold.
    fn free_name(&self, document: &ItemRef, stem: &str, export: &str) -> Option<ItemRef> {
        let state = self.lock();
        (1..)
            .map(|n| match n {
                1 => format!("{stem}.{export}"),
                n => format!("{stem} {n}.{export}"),
            })
            .filter_map(|text| files_core::FileName::new(text).ok())
            .map(|name| document.renamed(name))
            .find(|candidate| !state.items.contains_key(candidate))
    }

    fn check(&self) -> Result<(), CloudError> {
        if self.lock().away {
            Err(CloudError::Unavailable)
        } else {
            Ok(())
        }
    }
}
