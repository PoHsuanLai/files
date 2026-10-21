//! The errors of this crate that are not about launching.

use files_core::{EjectFailure, MountFailure};

/// Why the disks could not be listed or watched.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum DiskError {
    /// The disk service did not answer.
    #[error("The disks can't be listed right now: {0}")]
    Bus(#[from] zbus::Error),
}

/// Why the app's actions could not be offered to the assistant.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum IntentError {
    /// The bus would not take the service.
    #[error("Files couldn\u{2019}t offer its actions: {0}")]
    Offer(String),
    /// The shipped actions file is not one the assistant accepts. A build with a broken file
    /// never ships, so this is a packaging mistake.
    #[error("Files\u{2019} actions file is not valid: {0}")]
    Manifest(String),
}

/// What a disk's service said to an unmount, for those who decide what to tell the person.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum Said {
    /// Something has the disk open.
    Busy,
    /// Not allowed (or the prompt was dismissed).
    NotAllowed,
    /// It was not mounted to begin with.
    NotMounted,
    /// It was already mounted.
    AlreadyMounted,
    /// Anything else.
    Other,
}

impl Said {
    /// The meaning of a D-Bus error name from the disk service (`None` for an error that is not a
    /// method error).
    pub(crate) fn of(name: Option<&str>) -> Said {
        match name {
            Some("org.freedesktop.UDisks2.Error.DeviceBusy") => Said::Busy,
            Some(
                "org.freedesktop.UDisks2.Error.NotAuthorized"
                | "org.freedesktop.UDisks2.Error.NotAuthorizedCanObtain"
                | "org.freedesktop.UDisks2.Error.NotAuthorizedDismissed"
                | "org.freedesktop.DBus.Error.AccessDenied",
            ) => Said::NotAllowed,
            Some("org.freedesktop.UDisks2.Error.NotMounted") => Said::NotMounted,
            Some("org.freedesktop.UDisks2.Error.AlreadyMounted") => Said::AlreadyMounted,
            Some(_) | None => Said::Other,
        }
    }

    /// The same, for an error from a call.
    pub(crate) fn from_error(error: &zbus::Error) -> Said {
        match error {
            zbus::Error::MethodError(name, _, _) => Said::of(Some(name.as_str())),
            // Not an answer from the disk service: the bus itself failed.
            zbus::Error::FDO(_) | zbus::Error::Failure(_) | _ => Said::Other,
        }
    }

    /// As a failure to open.
    pub(crate) fn mount_failure(self) -> MountFailure {
        match self {
            Said::NotAllowed => MountFailure::NotAllowed,
            Said::Busy | Said::NotMounted | Said::AlreadyMounted | Said::Other => {
                MountFailure::Other
            }
        }
    }

    /// As a failure to eject, with the programs found when it is busy.
    pub(crate) fn eject_failure(self, programs: Vec<files_core::ProgramName>) -> EjectFailure {
        match self {
            Said::Busy => EjectFailure::InUse(programs),
            Said::NotAllowed => EjectFailure::NotAllowed,
            Said::NotMounted | Said::AlreadyMounted | Said::Other => EjectFailure::Other,
        }
    }
}

/// Why the window would not do what was asked.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum Refusal {
    /// The window is closed.
    #[error("Files isn\u{2019}t open any more.")]
    Closed,
    /// The change asked to undo is not the newest one any more.
    #[error("That change can\u{2019}t be undone any more.")]
    Gone,
    /// Anything else, in words.
    #[error("{0}")]
    Failed(String),
}
