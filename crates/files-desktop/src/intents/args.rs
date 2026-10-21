//! Reading a call's target and arguments into an [`Ask`]. The router has already checked them
//! against the manifest (type, range); this reads them into Files' own types and refuses what a
//! window cannot hold.

use std::path::{Path, PathBuf};

use docket_core::{AppRefusal, Args, FailText, Invocation, ParamName, TargetValue, Value};
use files_core::{FileName, FolderPath, ItemRef};

use super::ask::Ask;
use super::names;

fn failed(words: &str) -> AppRefusal {
    AppRefusal::Failed(FailText(words.to_owned()))
}

fn missing(name: &str) -> AppRefusal {
    match ParamName::parse(name) {
        Ok(param) => AppRefusal::NeedsParam {
            param,
            options: Vec::new(),
        },
        Err(_) => AppRefusal::Unsupported,
    }
}

/// The text of a value that is a file or words.
fn text_of(value: &Value) -> Option<&str> {
    if let Value::File(file) = value {
        return Some(file.as_str());
    }
    if let Value::Text(text) = value {
        return Some(text.as_str());
    }
    None
}

/// The text of the argument `name`, as a file or as words.
fn argument<'a>(args: &'a Args, name: &str) -> Result<Option<&'a str>, AppRefusal> {
    match args.iter().find(|(key, _)| key.as_str() == name) {
        None => Ok(None),
        Some((_, labelled)) => text_of(&labelled.value)
            .map(Some)
            .ok_or_else(|| failed("a file or folder was expected")),
    }
}

fn path(args: &Args, name: &str) -> Result<PathBuf, AppRefusal> {
    let text = argument(args, name)?.ok_or_else(|| missing(name))?;
    let path = Path::new(text);
    if path.is_absolute() {
        Ok(path.to_path_buf())
    } else {
        Err(failed("give the path in full, from the root"))
    }
}

fn folder(args: &Args, name: &str) -> Result<FolderPath, AppRefusal> {
    FolderPath::new(path(args, name)?).map_err(|_| failed("give the folder in full"))
}

fn items(target: &TargetValue) -> Result<Vec<ItemRef>, AppRefusal> {
    let TargetValue::Files(files) = target else {
        return Err(failed("name the files to work on"));
    };
    let items: Vec<ItemRef> = files
        .iter()
        .map(|file| ItemRef::from_path(Path::new(file.as_str())))
        .collect::<Result<_, _>>()
        .map_err(|_| failed("give each file in full, from the root"))?;
    if items.is_empty() {
        return Err(failed("name the files to work on"));
    }
    Ok(items)
}

fn name(args: &Args) -> Result<Option<FileName>, AppRefusal> {
    match argument(args, "name")? {
        None | Some("") => Ok(None),
        Some(text) => FileName::new(text)
            .map(Some)
            .map_err(|error| failed(&error.to_string())),
    }
}

/// What the call asks of the window. `Unsupported` for an action that is not ours.
pub(super) fn ask_of(call: &Invocation) -> Result<Ask, AppRefusal> {
    let args = &call.args;
    match call.action.as_str() {
        names::OPEN => Ok(Ask::Open(path(args, "path")?)),
        names::TRASH => Ok(Ask::Trash(items(&call.target)?)),
        names::NEW_FOLDER => Ok(Ask::NewFolder {
            parent: folder(args, "folder")?,
            name: name(args)?,
        }),
        names::COPY => Ok(Ask::Copy {
            sources: items(&call.target)?,
            to: folder(args, "to")?,
        }),
        names::MOVE => Ok(Ask::Move {
            sources: items(&call.target)?,
            to: folder(args, "to")?,
        }),
        names::EMPTY_TRASH => Ok(Ask::EmptyTrash),
        names::SELECTION => Ok(Ask::Selection),
        names::CURRENT_FOLDER => Ok(Ask::CurrentFolder),
        _ => Err(AppRefusal::Unsupported),
    }
}
