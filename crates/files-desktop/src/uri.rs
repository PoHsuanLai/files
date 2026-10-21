//! The addresses other programs hand a file manager: `file:///…` and `trash:///`.
//!
//! Pure functions over text. A `file:` address is percent-decoded into the path's own bytes (Linux
//! names are bytes, not text), so a name that is not UTF-8 survives the round trip.
//!
//! ```
//! use files_desktop::uri::{Uri, UriError, file_uri, parse};
//! use std::path::Path;
//!
//! let uri = parse("file:///home/me/My%20Files/caf%C3%A9.txt")?;
//! assert_eq!(uri, Uri::File("/home/me/My Files/café.txt".into()));
//! assert_eq!(file_uri(Path::new("/home/me/My Files")), "file:///home/me/My%20Files");
//! assert_eq!(parse("https://example.org/"), Err(UriError::Scheme("https".to_owned())));
//! # Ok::<(), UriError>(())
//! ```

use std::ffi::OsString;
use std::os::unix::ffi::{OsStrExt, OsStringExt};
use std::path::{Path, PathBuf};

/// Where an address points.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub enum Uri {
    /// A path on this computer.
    File(PathBuf),
    /// The Trash.
    Trash,
}

/// Why an address was refused, in the words a person reads.
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UriError {
    /// Not an address at all: no `scheme:` in front.
    #[error("That isn\u{2019}t an address.")]
    NoScheme,
    /// An address of a kind files does not open, such as `https:`.
    #[error("Files can\u{2019}t open \u{201c}{0}:\u{201d} addresses.")]
    Scheme(String),
    /// A `file:` address on another computer.
    #[error("That file is on another computer, \u{201c}{0}\u{201d}.")]
    Remote(String),
    /// A `file:` address whose path does not start at the root.
    #[error("That address doesn\u{2019}t say where the file is.")]
    Relative,
    /// A `%` not followed by two hexadecimal digits.
    #[error("That address has a broken \u{201c}%\u{201d} code.")]
    BadEscape,
    /// A NUL byte, which no Linux path may hold.
    #[error("That address has a null character.")]
    Nul,
}

/// What `text` points at.
pub fn parse(text: &str) -> Result<Uri, UriError> {
    let (scheme, rest) = split_scheme(text).ok_or(UriError::NoScheme)?;
    match scheme.to_ascii_lowercase().as_str() {
        "file" => parse_file(rest).map(Uri::File),
        "trash" => Ok(Uri::Trash),
        other => Err(UriError::Scheme(other.to_owned())),
    }
}

/// `scheme` and the rest, when `text` starts with a valid scheme and a colon.
fn split_scheme(text: &str) -> Option<(&str, &str)> {
    let (scheme, rest) = text.split_once(':')?;
    let mut chars = scheme.chars();
    let first = chars.next()?;
    let valid = first.is_ascii_alphabetic()
        && chars.all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '-' | '.'));
    valid.then_some((scheme, rest))
}

fn parse_file(rest: &str) -> Result<PathBuf, UriError> {
    // A query or fragment is not part of a path; a `#` in a name arrives as `%23`.
    let rest = rest.split(['?', '#']).next().unwrap_or_default();
    let (authority, path) = match rest.strip_prefix("//") {
        Some(after) => match after.find('/') {
            Some(at) => after.split_at(at),
            None => (after, ""),
        },
        None => ("", rest),
    };
    if !(authority.is_empty() || authority.eq_ignore_ascii_case("localhost")) {
        return Err(UriError::Remote(authority.to_owned()));
    }
    if !path.starts_with('/') {
        return Err(UriError::Relative);
    }
    let bytes = decode(path)?;
    if bytes.contains(&0) {
        return Err(UriError::Nul);
    }
    // Collecting the components drops a trailing slash and doubled ones.
    Ok(PathBuf::from(OsString::from_vec(bytes))
        .components()
        .collect())
}

fn decode(text: &str) -> Result<Vec<u8>, UriError> {
    let mut out = Vec::with_capacity(text.len());
    let mut bytes = text.bytes();
    while let Some(byte) = bytes.next() {
        if byte != b'%' {
            out.push(byte);
            continue;
        }
        let high = bytes.next().and_then(hex);
        let low = bytes.next().and_then(hex);
        match (high, low) {
            (Some(high), Some(low)) => out.push((high << 4) | low),
            _ => return Err(UriError::BadEscape),
        }
    }
    Ok(out)
}

fn hex(byte: u8) -> Option<u8> {
    char::from(byte)
        .to_digit(16)
        .and_then(|digit| u8::try_from(digit).ok())
}

/// The `file:` address of `path`: every byte but letters, digits, `/` and `-._~` is `%XX`.
pub fn file_uri(path: &Path) -> String {
    let mut out = String::from("file://");
    for &byte in path.as_os_str().as_bytes() {
        if byte.is_ascii_alphanumeric() || matches!(byte, b'/' | b'-' | b'.' | b'_' | b'~') {
            out.push(char::from(byte));
        } else {
            out.push_str(&format!("%{byte:02X}"));
        }
    }
    out
}

/// The address of the Trash.
pub const TRASH_URI: &str = "trash:///";
