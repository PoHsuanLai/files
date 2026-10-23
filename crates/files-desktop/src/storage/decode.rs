//! The service's rows as files-core's vocabulary. An unknown word is an error, except a drive's
//! state, which the service says a caller shows as it shows an error.

use files_core::{
    AccountLabel, ByteSize, CloudDrive, CloudItem, CloudKind, CloudState, CloudWords, DriveId,
    DriveState, FolderPath, ItemRef, Keeping, OnlineUrl, ProviderName, Refused, Share, Usage,
};
use porter_dbus::{
    DRIVE_KEY_LABEL, DRIVE_KEY_LOCAL, DRIVE_KEY_PATH, DRIVE_KEY_PROVIDER, DRIVE_KEY_STATE,
    DRIVE_KEY_TOTAL, DRIVE_KEY_USED, DRIVE_KEY_WORDS, Details, ITEM_KEY_FRACTION, ITEM_KEY_KEPT,
    ITEM_KEY_KIND, ITEM_KEY_OTHER, ITEM_KEY_STATE, ITEM_KEY_URL, ITEM_KEY_WORDS, ItemKind,
    ItemState, Refused as WireRefused,
};

use super::error::StorageError;

fn text(details: &Details, key: &str) -> Option<String> {
    details
        .get(key)
        .and_then(|value| <&str>::try_from(value).ok())
        .map(str::to_owned)
}

fn number(details: &Details, key: &str) -> Option<u64> {
    details.get(key).and_then(|value| u64::try_from(value).ok())
}

fn real(details: &Details, key: &str) -> Option<f64> {
    details.get(key).and_then(|value| f64::try_from(value).ok())
}

fn flag(details: &Details, key: &str) -> Option<bool> {
    details
        .get(key)
        .and_then(|value| bool::try_from(value).ok())
}

fn wire_state(word: Option<String>) -> DriveState {
    let state = word.and_then(|word| porter_dbus::DriveState::from_word(&word));
    match state {
        Some(porter_dbus::DriveState::Starting) => DriveState::Starting,
        Some(porter_dbus::DriveState::Ready) => DriveState::Ready,
        Some(porter_dbus::DriveState::Syncing) => DriveState::Syncing,
        Some(porter_dbus::DriveState::Offline) => DriveState::Offline,
        Some(porter_dbus::DriveState::Paused) => DriveState::Paused,
        Some(porter_dbus::DriveState::NeedsSignIn) => DriveState::NeedsSignIn,
        Some(porter_dbus::DriveState::Error) | Some(_) | None => DriveState::Error,
    }
}

/// One row of `Drives`: the drive's id and its details.
pub(super) fn drive(id: String, details: &Details) -> Result<CloudDrive, StorageError> {
    let provider = text(details, DRIVE_KEY_PROVIDER).ok_or(StorageError::Missing("provider"))?;
    let path = text(details, DRIVE_KEY_PATH).ok_or(StorageError::Missing("folder"))?;
    let folder = FolderPath::new(&path).map_err(|_| StorageError::Path(path))?;
    let usage = Usage {
        used: number(details, DRIVE_KEY_USED).map(ByteSize),
        total: number(details, DRIVE_KEY_TOTAL).map(ByteSize),
        local: ByteSize(number(details, DRIVE_KEY_LOCAL).unwrap_or(0)),
    };
    let drive = CloudDrive::new(DriveId::new(id), ProviderName::new(provider), folder)
        .labelled(AccountLabel::new(
            text(details, DRIVE_KEY_LABEL).unwrap_or_default(),
        ))
        .in_state(wire_state(text(details, DRIVE_KEY_STATE)))
        .using(usage);
    Ok(match text(details, DRIVE_KEY_WORDS) {
        Some(words) => drive.saying(CloudWords::new(words)),
        None => drive,
    })
}

fn state_of(word: &str) -> Result<CloudState, StorageError> {
    match ItemState::from_word(word) {
        Some(ItemState::Cloud) => Ok(CloudState::Cloud),
        Some(ItemState::Downloading) => Ok(CloudState::Downloading),
        Some(ItemState::Downloaded) => Ok(CloudState::Downloaded),
        Some(ItemState::Uploading) => Ok(CloudState::Uploading),
        Some(ItemState::Waiting) => Ok(CloudState::Waiting),
        Some(ItemState::Error) => Ok(CloudState::Error),
        Some(ItemState::Conflict) => Ok(CloudState::Conflict),
        Some(_) | None => Err(StorageError::Unknown {
            part: "state",
            word: word.to_owned(),
        }),
    }
}

fn kind_of(word: &str) -> Result<CloudKind, StorageError> {
    match ItemKind::from_word(word) {
        Some(ItemKind::File) => Ok(CloudKind::File),
        Some(ItemKind::Folder) => Ok(CloudKind::Folder),
        Some(ItemKind::Online) => Ok(CloudKind::Online),
        Some(_) | None => Err(StorageError::Unknown {
            part: "kind",
            word: word.to_owned(),
        }),
    }
}

fn item_ref(path: &str) -> Result<ItemRef, StorageError> {
    ItemRef::from_path(std::path::Path::new(path)).map_err(|_| StorageError::Path(path.to_owned()))
}

/// One entry of `Items` or `ItemsChanged`: a path and its details.
pub(super) fn item(path: &str, details: &Details) -> Result<(ItemRef, CloudItem), StorageError> {
    let item = item_ref(path)?;
    let state = text(details, ITEM_KEY_STATE).ok_or(StorageError::Missing("state"))?;
    let kind = text(details, ITEM_KEY_KIND).ok_or(StorageError::Missing("kind"))?;
    let mut found = CloudItem::new(kind_of(&kind)?, state_of(&state)?);
    if flag(details, ITEM_KEY_KEPT) == Some(true) {
        found = found.kept();
    }
    if let Some(fraction) = real(details, ITEM_KEY_FRACTION) {
        found = found.moving(Share::from_ratio(fraction));
    }
    if let Some(words) = text(details, ITEM_KEY_WORDS) {
        found = found.failed(CloudWords::new(words));
    }
    if let Some(url) = text(details, ITEM_KEY_URL).and_then(|url| OnlineUrl::new(url).ok()) {
        found = found.opening_at(url);
    }
    if let Some(other) = text(details, ITEM_KEY_OTHER).and_then(|other| item_ref(&other).ok()) {
        found = found.beside(other);
    }
    Ok((item, found))
}

/// What `Keeping` is on the wire: the flag `Keep` takes.
pub(super) fn keeps(keeping: Keeping) -> bool {
    match keeping {
        Keeping::Kept => true,
        Keeping::NotKept => false,
    }
}

/// The refused paths of an ask, in path order.
pub(super) fn refused(wire: WireRefused) -> Vec<Refused> {
    let mut rows: Vec<(String, String)> = wire.into_iter().collect();
    rows.sort();
    rows.into_iter()
        .filter_map(|(path, words)| {
            Some(Refused {
                item: item_ref(&path).ok()?,
                words: CloudWords::new(words),
            })
        })
        .collect()
}
