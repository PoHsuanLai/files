//! The command line: `files [FOLDER]`.
//!
//! `FOLDER` is a path or a `file://` address (the desktop launches `files %U`). More than one is
//! allowed and only the first is opened: one window shows one folder.

use std::ffi::OsString;
use std::path::PathBuf;

/// Where the window starts.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Start {
    /// The person's home folder.
    Home,
    /// This folder (a relative path is relative to the working folder).
    Given(PathBuf),
}

/// What the command line asked for.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Args {
    /// Where to start.
    pub start: Start,
}

impl Args {
    /// Reads the arguments after the program's name; the error is a sentence for the person.
    pub fn parse(args: impl IntoIterator<Item = OsString>) -> Result<Args, String> {
        let mut folders = Vec::new();
        for arg in args {
            if arg.to_string_lossy().starts_with('-') && arg != "-" {
                return Err(format!(
                    "I don't know the option {}.",
                    arg.to_string_lossy()
                ));
            }
            folders.push(arg);
        }
        let start = match folders.into_iter().next() {
            Some(first) => Start::Given(path_of(&first)),
            None => Start::Home,
        };
        Ok(Args { start })
    }
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
        })
    }

    #[test]
    fn the_command_line_names_a_folder_or_nothing() {
        let cases = [
            ("nothing", parse(&[]), Ok(Args { start: Start::Home })),
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
        ];
        for (name, got, want) in cases {
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
