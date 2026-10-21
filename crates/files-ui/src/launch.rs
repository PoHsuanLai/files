//! Everything the window is given at start.

use std::path::PathBuf;
use std::sync::Arc;

use tokio::runtime::Handle;

use ds::prelude::SystemPrefs;
use ds_settings::{AppName, ConfigRoot, Store, SystemPrefsSource};
use files_core::Location;

use crate::edge::{Opener, Seams, Zone};
use crate::places::Places;
use crate::requests::Requests;

/// Where the look comes from: the design system's own settings (`appearance.toml`: the person's
/// theme, accent and motion) and the desktop's preferences. The window follows both live.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppearanceSource {
    /// The settings folder.
    pub store: Store,
    /// Where the desktop's preferences are read from.
    pub system: SystemPrefsSource,
}

impl AppearanceSource {
    /// The person's own: `$XDG_CONFIG_HOME/quire/appearance.toml` and the desktop's settings
    /// portal.
    pub fn desktop() -> AppearanceSource {
        AppearanceSource {
            store: Store::new(ConfigRoot::Xdg, AppName::QUIRE),
            system: SystemPrefsSource::Portal,
        }
    }

    /// A scratch config folder and fixed preferences, for tests and headless runs.
    pub fn scratch(config_dir: PathBuf, system: SystemPrefs) -> AppearanceSource {
        AppearanceSource {
            store: Store::new(ConfigRoot::Scratch(config_dir), AppName::QUIRE),
            system: SystemPrefsSource::Fixed(system),
        }
    }
}

/// What a window starts from: where to look, the favourite places, the look, the time zone and
/// the seams to the outside. Built once by the binary from the environment, handed to the window
/// as a root context, and read by nothing else.
#[derive(Debug, Clone)]
pub struct Launch {
    /// The place the window opens on.
    pub start: Location,
    /// The sidebar's favourites.
    pub places: Places,
    /// Where the look comes from.
    pub appearance: AppearanceSource,
    /// The zone dates are shown in.
    pub zone: Zone,
    /// The disk and the desktop, behind traits.
    pub seams: Seams,
    /// What other programs ask of the window (show this folder, reveal these files).
    pub requests: Requests,
    /// The runtime the window runs on, when the caller made one (the one that also drives its
    /// bus connection); `None` lets the window make its own.
    pub runtime: Option<Handle>,
}

impl Launch {
    /// A window on `start` over the real disk, opening files with `opener`, with the person's own
    /// look and time zone and no favourites.
    pub fn new(start: Location, opener: Arc<dyn Opener>) -> Launch {
        Launch {
            start,
            places: Places::default(),
            appearance: AppearanceSource::desktop(),
            zone: Zone::Local,
            seams: Seams::disk(opener),
            requests: Requests::none(),
            runtime: None,
        }
    }

    /// The same launch with these favourites.
    pub fn with_places(self, places: Places) -> Launch {
        Launch { places, ..self }
    }

    /// The same launch with this look.
    pub fn with_appearance(self, appearance: AppearanceSource) -> Launch {
        Launch { appearance, ..self }
    }

    /// The same launch showing dates in `zone`.
    pub fn with_zone(self, zone: Zone) -> Launch {
        Launch { zone, ..self }
    }

    /// The same launch answering these requests.
    pub fn with_requests(self, requests: Requests) -> Launch {
        Launch { requests, ..self }
    }

    /// The same launch running on the runtime `handle` belongs to.
    pub fn with_runtime(self, handle: Handle) -> Launch {
        Launch {
            runtime: Some(handle),
            ..self
        }
    }

    /// The same launch over these seams.
    pub fn with_seams(self, seams: Seams) -> Launch {
        Launch { seams, ..self }
    }
}
