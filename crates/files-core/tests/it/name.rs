//! What a name may be, and how it splits into stem and extension.

use files_core::{FileName, NameError};
use std::ffi::{OsStr, OsString};
use std::os::unix::ffi::OsStringExt;

#[test]
fn a_name_is_refused_for_what_linux_refuses() {
    let long = "a".repeat(256);
    let cases: &[(&str, Result<(), NameError>)] = &[
        ("notes.txt", Ok(())),
        ("", Err(NameError::Empty)),
        (".", Err(NameError::Reserved)),
        ("..", Err(NameError::Reserved)),
        ("...", Ok(())),
        ("a/b", Err(NameError::Slash)),
        ("a\0b", Err(NameError::Nul)),
        (&long, Err(NameError::TooLong)),
        (&long[..255], Ok(())),
    ];
    for (text, want) in cases {
        assert_eq!(FileName::new(*text).map(drop), *want, "{text:?}");
    }
}

#[test]
fn the_extension_is_after_the_last_dot_unless_the_dot_leads_or_ends() {
    let cases: &[(&str, &str, Option<&str>)] = &[
        ("Report.pdf", "Report", Some("pdf")),
        ("archive.tar.gz", "archive.tar", Some("gz")),
        (".bashrc", ".bashrc", None),
        ("Makefile", "Makefile", None),
        ("trailing.", "trailing.", None),
    ];
    for (text, stem, ext) in cases {
        let name = FileName::new(*text).unwrap();
        assert_eq!(name.stem(), OsStr::new(stem), "{text}");
        assert_eq!(name.extension(), ext.map(OsStr::new), "{text}");
    }
}

#[test]
fn a_name_that_is_not_utf8_is_kept_and_shown_with_a_replacement() {
    let raw = OsString::from_vec(b"caf\xe9.txt".to_vec());
    let name = FileName::new(raw.clone()).unwrap();
    assert_eq!(name.as_os_str(), raw.as_os_str());
    assert_eq!(name.display(), "caf\u{fffd}.txt");
    assert_eq!(name.extension(), Some(OsStr::new("txt")));
}
