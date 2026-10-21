//! The window, as Files' actions for agents see it: each ask becomes a model input sent through
//! the window's request channel, so a change is queued, shown and undone like the person's own.
//! The two reads and the undo wait for the window's answer.

use std::future::Future;
use std::path::{Path, PathBuf};

use files_core::{FileName, FolderPath, ItemRef, Location};
use files_desktop::{Ask, Refusal, Reply, Undoing, Window};
use files_model::{Bring, Input};
use files_ui::{Query, ReadError, Reading, Request, RequestSender};

/// What the window is to do with a path an agent named.
pub fn open_input(path: &Path) -> Result<Input, Refusal> {
    let folder = FolderPath::new(path).map_err(|_| not_there())?;
    let meta = std::fs::metadata(path).map_err(|_| not_there())?;
    if meta.is_dir() {
        return Ok(Input::Go(Location::Folder(folder)));
    }
    let parent = path
        .parent()
        .and_then(|parent| FolderPath::new(parent).ok())
        .ok_or_else(not_there)?;
    let name = path
        .file_name()
        .and_then(|name| FileName::new(name).ok())
        .ok_or_else(not_there)?;
    Ok(Input::Reveal {
        folder: parent,
        names: vec![name],
    })
}

fn not_there() -> Refusal {
    Refusal::Failed("That file or folder isn\u{2019}t there.".to_owned())
}

fn closed(_: ReadError) -> Refusal {
    Refusal::Closed
}

fn paths_of(items: Vec<ItemRef>) -> Vec<PathBuf> {
    items.iter().map(ItemRef::path).collect()
}

/// The window behind its request channel.
#[derive(Debug, Clone)]
pub struct Port {
    sender: RequestSender,
}

impl Port {
    /// The window that `sender` reaches.
    pub fn new(sender: RequestSender) -> Port {
        Port { sender }
    }

    async fn read(&self, query: Query) -> Result<Reading, Refusal> {
        self.sender
            .ask(query)
            .map_err(closed)?
            .wait()
            .await
            .map_err(closed)
    }

    fn quietly(&self, input: Input) -> Result<Reply, Refusal> {
        self.sender.send_quietly(input);
        Ok(Reply::Done)
    }

    /// Takes back the newest change, if it is the one `undoing` describes.
    async fn undo(&self, undoing: Undoing) -> Result<Reply, Refusal> {
        let next = match self.read(Query::NextUndo).await? {
            Reading::Undo(next) => next,
            Reading::Selection(_) | Reading::Place(_) | _ => None,
        };
        match next {
            Some(next) if next.kind == undoing.kind && next.count == undoing.count => {
                self.quietly(Input::Undo)
            }
            Some(_) | None => Err(Refusal::Gone),
        }
    }

    async fn carry_out(&self, ask: Ask) -> Result<Reply, Refusal> {
        match ask {
            Ask::Open(path) => {
                let input = open_input(&path)?;
                self.sender.send(Request { input, raise: None });
                Ok(Reply::Done)
            }
            Ask::Trash(items) => self.quietly(Input::TrashItems(items)),
            Ask::NewFolder { parent, name } => self.quietly(Input::MakeFolder { parent, name }),
            Ask::Copy { sources, to } => self.quietly(Input::Bring {
                sources,
                destination: to,
                how: Bring::Copy,
            }),
            Ask::Move { sources, to } => self.quietly(Input::Bring {
                sources,
                destination: to,
                how: Bring::Move,
            }),
            Ask::EmptyTrash => {
                // The person is asked in the window, which comes forward for it.
                let input = Input::AskEmptyTrash;
                self.sender.send(Request { input, raise: None });
                Ok(Reply::Done)
            }
            Ask::Selection => match self.read(Query::Selection).await? {
                Reading::Selection(items) => Ok(Reply::Items(paths_of(items))),
                Reading::Place(_) | Reading::Undo(_) | _ => Err(Refusal::Closed),
            },
            Ask::CurrentFolder => match self.read(Query::CurrentFolder).await? {
                Reading::Place(place) => Ok(Reply::Place(place)),
                Reading::Selection(_) | Reading::Undo(_) | _ => Err(Refusal::Closed),
            },
            Ask::Undo(undoing) => self.undo(undoing).await,
            // An ask files-desktop adds later is not carried out until this names it.
            _ => Err(Refusal::Failed(
                "Files can\u{2019}t do that yet.".to_owned(),
            )),
        }
    }
}

impl Window for Port {
    fn ask(&self, ask: Ask) -> impl Future<Output = Result<Reply, Refusal>> + Send {
        let port = self.clone();
        async move { port.carry_out(ask).await }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_folder_is_gone_to_and_a_file_is_pointed_at_and_nothing_else_is_shown() {
        let dir = tempfile::tempdir().unwrap();
        std::fs::write(dir.path().join("a.txt"), "x").unwrap();
        let folder = FolderPath::new(dir.path()).unwrap();
        let cases = [
            (
                "a folder",
                dir.path().to_path_buf(),
                Some(Input::Go(Location::Folder(folder.clone()))),
            ),
            (
                "a file",
                dir.path().join("a.txt"),
                Some(Input::Reveal {
                    folder,
                    names: vec![FileName::new("a.txt").unwrap()],
                }),
            ),
            ("a path that is not there", dir.path().join("nope"), None),
            ("a relative path", PathBuf::from("a.txt"), None),
        ];
        for (label, path, want) in cases {
            assert_eq!(open_input(&path).ok(), want, "{label}");
        }
    }
}
