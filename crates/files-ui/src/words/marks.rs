//! What the small marks of a cloud drive say to assistive technology.

/// An item that is only in the cloud.
pub const IN_THE_CLOUD: &str = "In the cloud";
/// An item being downloaded.
pub const DOWNLOADING: &str = "Downloading";
/// An item being sent.
pub const UPLOADING: &str = "Uploading";
/// An item changed here and not yet sent.
pub const WAITING: &str = "Waiting to upload";
/// An item that could not be sent or fetched.
pub const FAILED: &str = "Couldn\u{2019}t sync";
/// An item changed here and on the drive both.
pub const CONFLICT: &str = "Two versions";
/// An online document.
pub const ONLINE: &str = "Opens in your browser";
/// A drive that is syncing.
pub const SYNCING: &str = "Syncing";
/// A drive that is offline.
pub const OFFLINE: &str = "Offline";
/// A drive whose sync is paused.
pub const PAUSED: &str = "Syncing paused";
/// A drive that needs the person to sign in.
pub const NEEDS_SIGN_IN: &str = "Needs sign-in";
/// A drive in an error state.
pub const DRIVE_FAILED: &str = "Something went wrong";
