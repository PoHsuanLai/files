//! Addresses: file and trash, and everything that is refused.

use std::ffi::OsString;
use std::os::unix::ffi::OsStringExt;
use std::path::{Path, PathBuf};

use files_desktop::uri::{TRASH_URI, Uri, UriError, file_uri, parse};

fn file(path: &str) -> Result<Uri, UriError> {
    Ok(Uri::File(PathBuf::from(path)))
}

#[test]
fn addresses_are_read() {
    let rows = [
        ("file:///home/me/a.txt", file("/home/me/a.txt")),
        (
            "file:///home/me/My%20Files/a%20b.txt",
            file("/home/me/My Files/a b.txt"),
        ),
        ("file:///tmp/caf%C3%A9", file("/tmp/café")),
        ("file:///tmp/café", file("/tmp/café")),
        ("file:///tmp/%E6%97%A5%E6%9C%AC", file("/tmp/日本")),
        ("file://localhost/tmp/x", file("/tmp/x")),
        ("file://LOCALHOST/tmp/x", file("/tmp/x")),
        ("file:/tmp/x", file("/tmp/x")),
        ("FILE:///tmp/x", file("/tmp/x")),
        ("file:///tmp/dir/", file("/tmp/dir")),
        ("file:///tmp//dir", file("/tmp/dir")),
        ("file:///tmp/a%23b", file("/tmp/a#b")),
        ("file:///tmp/a%2fb", file("/tmp/a/b")),
        ("file:///tmp/a?x=1#frag", file("/tmp/a")),
        ("file:///", file("/")),
        ("trash:///", Ok(Uri::Trash)),
        ("trash:///a.txt", Ok(Uri::Trash)),
        ("TRASH:", Ok(Uri::Trash)),
    ];
    for (text, want) in rows {
        assert_eq!(parse(text), want, "{text}");
    }
}

#[test]
fn addresses_that_cannot_be_shown_are_refused() {
    let rows = [
        ("", UriError::NoScheme),
        ("garbage", UriError::NoScheme),
        ("/tmp/x", UriError::NoScheme),
        ("relative/path", UriError::NoScheme),
        ("1file:///tmp", UriError::NoScheme),
        ("file:relative", UriError::Relative),
        ("file://", UriError::Relative),
        ("file://host/tmp/x", UriError::Remote("host".to_owned())),
        ("https://example.org/", UriError::Scheme("https".to_owned())),
        ("sftp://me@host/x", UriError::Scheme("sftp".to_owned())),
        ("file:///tmp/%zz", UriError::BadEscape),
        ("file:///tmp/%2", UriError::BadEscape),
        ("file:///tmp/%", UriError::BadEscape),
        ("file:///tmp/a%00b", UriError::Nul),
    ];
    for (text, want) in rows {
        assert_eq!(parse(text), Err(want), "{text}");
    }
}

#[test]
fn writing_then_reading_gives_the_path_back_even_for_a_name_that_is_not_text() {
    let paths = [
        "/",
        "/tmp/a b/é#%.txt",
        "/home/me/100% [done] (1).txt",
        "/a/~b/c-d_e.f",
    ];
    for path in paths {
        let written = file_uri(Path::new(path));
        assert!(written.starts_with("file:///"), "{written}");
        assert!(!written.contains(' '), "{written}");
        assert_eq!(parse(&written), file(path), "{path}");
    }
    assert_eq!(parse(TRASH_URI), Ok(Uri::Trash));

    let bytes = b"/tmp/\xff\xfe.bin".to_vec();
    let path = PathBuf::from(OsString::from_vec(bytes));
    assert_eq!(file_uri(&path), "file:///tmp/%FF%FE.bin");
    assert_eq!(parse("file:///tmp/%FF%FE.bin"), Ok(Uri::File(path)));
}
