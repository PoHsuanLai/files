//! What a caller asked for, as typed data.

use files_core::{FileName, FolderPath, Location};

use crate::uri::{self, Uri};

/// The caller's startup id (an activation token), when it gave one.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StartupId(String);

impl StartupId {
    /// The id `text`; empty means the caller gave none, so `None`.
    pub fn new(text: &str) -> Option<StartupId> {
        (!text.is_empty()).then(|| StartupId(text.to_owned()))
    }

    /// The id as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}

/// One file to point at: the folder to open and the name to select in it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Reveal {
    /// The folder it is in.
    pub folder: FolderPath,
    /// Its name there.
    pub name: FileName,
}

/// What to show.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ShowRequest {
    /// Open each place in a window.
    Folders(Vec<Location>),
    /// Open each file's folder with the file selected.
    Items(Vec<Reveal>),
    /// Show each file's information panel.
    Properties(Vec<Reveal>),
}

/// Which of the three calls it was.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ShowKind {
    /// `ShowFolders`.
    Folders,
    /// `ShowItems`.
    Items,
    /// `ShowItemProperties`.
    Properties,
}

/// A request and the token to raise the window with.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Show {
    /// What to show.
    pub request: ShowRequest,
    /// The caller's activation token, if any.
    pub startup: Option<StartupId>,
}

impl Show {
    /// The request of a bus call. Addresses that cannot be shown (other kinds, other computers,
    /// broken ones) are skipped one by one; `None` when none was left.
    pub fn from_wire(kind: ShowKind, uris: &[String], startup_id: &str) -> Option<Show> {
        let parsed: Vec<Uri> = uris.iter().filter_map(|u| uri::parse(u).ok()).collect();
        let request = match kind {
            ShowKind::Folders => ShowRequest::Folders(parsed.iter().filter_map(location).collect()),
            ShowKind::Items => ShowRequest::Items(parsed.iter().filter_map(reveal).collect()),
            ShowKind::Properties => {
                ShowRequest::Properties(parsed.iter().filter_map(reveal).collect())
            }
        };
        (!request.is_empty()).then(|| Show {
            request,
            startup: StartupId::new(startup_id),
        })
    }
}

impl ShowRequest {
    /// How many places or files it names.
    pub fn len(&self) -> usize {
        match self {
            ShowRequest::Folders(places) => places.len(),
            ShowRequest::Items(files) | ShowRequest::Properties(files) => files.len(),
        }
    }

    /// Whether it names nothing.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

fn location(uri: &Uri) -> Option<Location> {
    match uri {
        Uri::File(path) => FolderPath::new(path.clone()).ok().map(Location::Folder),
        Uri::Trash => Some(Location::Trash),
    }
}

/// A file to select. The Trash and the root have no file to point at.
fn reveal(uri: &Uri) -> Option<Reveal> {
    match uri {
        Uri::File(path) => {
            let folder = FolderPath::new(path.parent()?).ok()?;
            let name = FileName::new(path.file_name()?).ok()?;
            Some(Reveal { folder, name })
        }
        Uri::Trash => None,
    }
}
