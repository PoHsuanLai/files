//! A cloud drive: one storage account shown as a folder.

use crate::{AccountLabel, ByteSize, CloudWords, DriveId, FolderPath, ProviderName};

/// Where a drive stands.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
#[non_exhaustive]
pub enum DriveState {
    /// Turned on and its listing not read yet.
    Starting,
    /// Up to date.
    Ready,
    /// Reading or sending changes.
    Syncing,
    /// No network: what is downloaded opens, the rest waits.
    Offline,
    /// The person paused it (in Settings or the shell).
    Paused,
    /// The account needs signing in; its words say why.
    NeedsSignIn,
    /// Something else is wrong; its words say what.
    Error,
}

/// Room, in bytes: what the account uses and may use, and what is on this computer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct Usage {
    /// What the account uses; `None` when the provider does not say.
    pub used: Option<ByteSize>,
    /// What the account may use; `None` when unknown or unlimited.
    pub total: Option<ByteSize>,
    /// What the drive's downloaded files take on this computer.
    pub local: ByteSize,
}

/// One drive the sidebar lists.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct CloudDrive {
    /// What the service calls it.
    pub id: DriveId,
    /// The provider, as people say it.
    pub provider: ProviderName,
    /// The account's own words; empty when there are none.
    pub label: AccountLabel,
    /// The folder its files are at; always read from the service, never built.
    pub folder: FolderPath,
    /// Where it stands.
    pub state: DriveState,
    /// Why it needs signing in, or what went wrong.
    pub words: Option<CloudWords>,
    /// What it uses.
    pub usage: Usage,
}

impl CloudDrive {
    /// A drive that is up to date, with no label, words or usage.
    pub fn new(id: DriveId, provider: ProviderName, folder: FolderPath) -> CloudDrive {
        CloudDrive {
            id,
            provider,
            label: AccountLabel::default(),
            folder,
            state: DriveState::Ready,
            words: None,
            usage: Usage::default(),
        }
    }

    /// The same drive for the account called `label`.
    pub fn labelled(self, label: AccountLabel) -> CloudDrive {
        CloudDrive { label, ..self }
    }

    /// The same drive in `state`.
    pub fn in_state(self, state: DriveState) -> CloudDrive {
        CloudDrive { state, ..self }
    }

    /// The same drive saying `words`.
    pub fn saying(self, words: CloudWords) -> CloudDrive {
        CloudDrive {
            words: Some(words),
            ..self
        }
    }

    /// The same drive using `usage`.
    pub fn using(self, usage: Usage) -> CloudDrive {
        CloudDrive { usage, ..self }
    }

    /// Whether `folder` is the drive's own folder or inside it.
    pub fn holds(&self, folder: &FolderPath) -> bool {
        folder.as_path().starts_with(self.folder.as_path())
    }

    /// The name the sidebar shows among `all` the drives: the provider, and the account's words
    /// after it when another drive is of the same provider.
    pub fn name_among(&self, all: &[CloudDrive]) -> String {
        let shared = all
            .iter()
            .any(|other| other.id != self.id && other.provider == self.provider);
        match (shared, self.label.as_str()) {
            (true, label) if !label.is_empty() => format!("{} ({label})", self.provider),
            (true, _) | (false, _) => self.provider.to_string(),
        }
    }
}
