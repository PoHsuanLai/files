//! What the bus asked, as what the window takes: a [`Show`] becomes a [`Request`].
//!
//! One window shows one folder, so a request that names several places is answered with the
//! first, and one that names files in several folders with the files of the first file's folder.
//! `ShowItemProperties` is `ShowItems` until Files has a Get Info panel.

use files_core::FolderPath;
use files_desktop::{Reveal, Show, ShowRequest};
use files_model::Input;
use files_ui::{RaiseToken, Request};

/// The request a bus call is; `None` when it names nothing the window can show.
pub fn request_of(show: Show) -> Option<Request> {
    let input = match show.request {
        ShowRequest::Folders(places) => Input::Go(places.into_iter().next()?),
        ShowRequest::Items(files) | ShowRequest::Properties(files) => reveal_input(files)?,
    };
    let raise = show.startup.map(|id| RaiseToken(id.as_str().to_owned()));
    Some(Request { input, raise })
}

/// Select the files that share the first file's folder.
fn reveal_input(files: Vec<Reveal>) -> Option<Input> {
    let folder: FolderPath = files.first()?.folder.clone();
    let names = files
        .into_iter()
        .filter(|file| file.folder == folder)
        .map(|file| file.name)
        .collect();
    Some(Input::Reveal { folder, names })
}

#[cfg(test)]
mod tests {
    use files_core::{FileName, Location};
    use files_desktop::StartupId;

    use super::*;

    fn folder(path: &str) -> FolderPath {
        FolderPath::new(path).unwrap()
    }

    fn reveal(path: &str, name: &str) -> Reveal {
        Reveal {
            folder: folder(path),
            name: FileName::new(name).unwrap(),
        }
    }

    fn show(request: ShowRequest) -> Show {
        Show {
            request,
            startup: StartupId::new("tok"),
        }
    }

    #[test]
    fn a_bus_call_becomes_the_input_the_browser_takes() {
        let raise = Some(RaiseToken("tok".to_owned()));
        let names = |list: &[&str]| list.iter().map(|n| FileName::new(n).unwrap()).collect();
        let cases = [
            (
                "first of several folders",
                show(ShowRequest::Folders(vec![
                    Location::Folder(folder("/a")),
                    Location::Folder(folder("/b")),
                ])),
                Some(Input::Go(Location::Folder(folder("/a")))),
            ),
            (
                "items of the first folder",
                show(ShowRequest::Items(vec![
                    reveal("/a", "x"),
                    reveal("/b", "y"),
                    reveal("/a", "z"),
                ])),
                Some(Input::Reveal {
                    folder: folder("/a"),
                    names: names(&["x", "z"]),
                }),
            ),
            (
                "properties are items",
                show(ShowRequest::Properties(vec![reveal("/a", "x")])),
                Some(Input::Reveal {
                    folder: folder("/a"),
                    names: names(&["x"]),
                }),
            ),
            ("nothing named", show(ShowRequest::Items(Vec::new())), None),
        ];
        for (label, shown, want) in cases {
            let got = request_of(shown);
            assert_eq!(got.as_ref().map(|r| &r.input), want.as_ref(), "{label}");
            assert_eq!(got.map(|r| r.raise), want.map(|_| raise.clone()), "{label}");
        }
    }
}
