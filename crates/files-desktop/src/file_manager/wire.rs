//! A [`Show`] back into the three bus calls, for the launch that hands its request on.

use files_core::Location;

use super::request::{Reveal, Show, ShowRequest};
use crate::uri::{TRASH_URI, file_uri};

/// One bus call: its method, addresses and startup id.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct Call {
    pub method: &'static str,
    pub uris: Vec<String>,
    pub startup_id: String,
}

impl Show {
    /// The call that says this.
    pub(crate) fn to_call(&self) -> Call {
        let (method, uris) = match &self.request {
            ShowRequest::Folders(places) => {
                ("ShowFolders", places.iter().filter_map(place_uri).collect())
            }
            ShowRequest::Items(files) => ("ShowItems", files.iter().map(reveal_uri).collect()),
            ShowRequest::Properties(files) => {
                ("ShowItemProperties", files.iter().map(reveal_uri).collect())
            }
        };
        Call {
            method,
            uris,
            startup_id: self
                .startup
                .as_ref()
                .map(|id| id.as_str().to_owned())
                .unwrap_or_default(),
        }
    }
}

fn place_uri(place: &Location) -> Option<String> {
    match place {
        Location::Folder(path) => Some(file_uri(path.as_path())),
        Location::Trash => Some(TRASH_URI.to_owned()),
        // A tag's search, and a place added later, has no address here until this names it.
        Location::Tag(_) | _ => None,
    }
}

fn reveal_uri(reveal: &Reveal) -> String {
    file_uri(&reveal.folder.as_path().join(reveal.name.as_os_str()))
}
