//! The command line: `files [--dbus-activated] [FOLDER]`.
//!
//! `FOLDER` is a path, a `file://` address (the desktop launches `files %U`) or `trash:///`, the
//! Trash. More than one is
//! allowed and only the first is opened: one window shows one folder. `--dbus-activated` is what
//! the session bus's service files pass: the bus started Files to answer a call, which then moves
//! the window, so nothing else is asked of an app that is already running.

use std::ffi::{OsStr, OsString};
use std::path::PathBuf;

/// Where the window starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// The person's home folder.
    Home,
    /// This folder (a relative path is relative to the working folder).
    Given(PathBuf),
    /// The Trash (`trash:///`, which the dock's Open Trash uses).
    Trash,
}

/// Who started this launch.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Started {
    /// The person (or a program acting for them) ran `files`.
    ByPerson,
    /// The session bus started it to answer a call.
    ByBus,
}

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// Where to start.
    pub start: Start,
    /// Who started it.
    pub started: Started,
}

impl Args {
    /// Reads the arguments after the program's name; the error is a sentence for the person.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
        let mut folders = Vec::new();
        let mut started = Started::ByPerson;
        for arg in args {
            if arg == "--dbus-activated" {
                started = Started::ByBus;
                continue;
            }
            if arg.to_string_lossy().starts_with('-') && arg != "-" {
                return Err(format!(
                    "I don't know the option {}.",
                    arg.to_string_lossy()
                ));
            }
            folders.push(arg);
        }
        let start = match folders.into_iter().next() {
            Some(first) if is_trash(&first) => Start::Trash,
            Some(first) => Start::Given(path_of(&first)),
            None => Start::Home,
        };
        Ok(Args { start, started })
    }
}

/// `files --write-schema DIR`: the folder to write the settings schema into, when that is what was
/// asked (it is the whole command line); the error is a sentence for the person.
pub fn schema_dir(args: &[OsString]) -> Option<Result<PathBuf, String>> {
    let (first, rest) = args.split_first()?;
    if first != OsStr::new("--write-schema") {
        return None;
    }
    Some(match rest {
        [dir] => Ok(PathBuf::from(dir)),
        _ => Err("--write-schema takes the one folder to write into.".to_owned()),
    })
}

/// Whether the argument is the address of the Trash: `trash:///`, or `trash:` with no path.
fn is_trash(arg: &OsString) -> bool {
    matches!(
        arg.to_string_lossy().as_ref(),
        "trash:///" | "trash://" | "trash:/" | "trash:"
    )
}

/// The path an argument names: a `file://` address is decoded, anything else is the path itself.
fn path_of(arg: &OsString) -> PathBuf {
    let text = arg.to_string_lossy();
    match text.strip_prefix("file://") {
        Some(rest) => {
            // `file:///home/p` has an empty host; `file://localhost/home/p` names this machine.
            let path = rest.strip_prefix("localhost").unwrap_or(rest);
            PathBuf::from(decoded(path))
        }
        None => PathBuf::from(arg),
    }
}

/// `text` with each `%XX` turned into its byte.
fn decoded(text: &str) -> OsString {
    use std::os::unix::ffi::OsStringExt;
    let bytes = text.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut at = 0;
    while let Some(&byte) = bytes.get(at) {
        let escaped = (byte == b'%')
            .then(|| bytes.get(at + 1..at + 3))
            .flatten()
            .and_then(|pair| std::str::from_utf8(pair).ok())
            .and_then(|pair| u8::from_str_radix(pair, 16).ok());
        match escaped {
            Some(value) => {
                out.push(value);
                at += 3;
            }
            None => {
                out.push(byte);
                at += 1;
            }
        }
    }
    OsString::from_vec(out)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(args: &[&str]) -> Result<Args, String> {
        Args::parse(args.iter().map(OsString::from))
    }

    fn given(path: &str) -> Result<Args, String> {
        Ok(Args {
            start: Start::Given(PathBuf::from(path)),
            started: Started::ByPerson,
        })
    }

    #[test]
    fn the_command_line_names_a_folder_or_nothing() {
        let cases = [
            (
                "nothing",
                parse(&[]),
                Ok(Args {
                    start: Start::Home,
                    started: Started::ByPerson,
                }),
            ),
            (
                "started by the bus",
                parse(&["--dbus-activated"]),
                Ok(Args {
                    start: Start::Home,
                    started: Started::ByBus,
                }),
            ),
            ("a path", parse(&["/tmp"]), given("/tmp")),
            ("a relative path", parse(&["notes"]), given("notes")),
            ("only the first", parse(&["/a", "/b"]), given("/a")),
            (
                "a file address",
                parse(&["file:///home/p/My%20Files"]),
                given("/home/p/My Files"),
            ),
            (
                "a local address",
                parse(&["file://localhost/tmp"]),
                given("/tmp"),
            ),
            (
                "a bad escape stays",
                parse(&["file:///a%zzb"]),
                given("/a%zzb"),
            ),
            ("a lone dash is a name", parse(&["-"]), given("-")),
            (
                "the trash",
                parse(&["trash:///"]),
                Ok(Args {
                    start: Start::Trash,
                    started: Started::ByPerson,
                }),
            ),
            (
                "the trash, first of several",
                parse(&["trash:///", "/tmp"]),
                Ok(Args {
                    start: Start::Trash,
                    started: Started::ByPerson,
                }),
            ),
        ];
        for (name, got, want) in cases {
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn writing_the_schema_names_one_folder() {
        let args = |list: &[&str]| list.iter().map(OsString::from).collect::<Vec<_>>();
        let cases = [
            (
                "a folder",
                args(&["--write-schema", "/tmp/s"]),
                Some(Ok("/tmp/s")),
            ),
            ("none", args(&["--write-schema"]), Some(Err(()))),
            ("two", args(&["--write-schema", "/a", "/b"]), Some(Err(()))),
            ("another command", args(&["/tmp"]), None),
            ("nothing", args(&[]), None),
        ];
        for (name, list, want) in cases {
            let got = schema_dir(&list).map(|asked| asked.map_err(|_| ()));
            let want = want.map(|asked: Result<&str, ()>| asked.map(PathBuf::from));
            assert_eq!(got, want, "{name}");
        }
    }

    #[test]
    fn an_option_is_refused_in_words() {
        assert_eq!(
            parse(&["--new-window"]),
            Err("I don't know the option --new-window.".to_owned())
        );
    }
}
