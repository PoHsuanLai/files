//! Name lookup: literal, suffix and pattern stages, weights, case, ambiguity, `__NOGLOBS__`.

use files_mime::{MimeDb, MimeLayer, NameGuess};

use crate::{mt, name};

const GLOBS: &str = "\
# fixture
50:text/plain:*.txt
50:text/x-readme:README
50:application/x-compressed-tar:*.tar.gz
50:application/gzip:*.gz
50:image/jpeg:*.jpg
50:text/x-makefile:Makefile
50:application/x-core:core.[0-9]*
50:image/x-raw:img_?.raw
50:text/x-special:special.txt
50:text/x-csrc:*.c
60:text/x-c++src:*.C:cs
50:text/x-weak:*.dup
70:text/x-strong:*.dup
50:text/x-amb-b:*.amb
50:text/x-amb-a:*.amb
50:text/x-prefixed:x.in.*t
50:text/x-neg:neg_[!a-c]
bad line
x:text/x-bad:*.bad
";

fn db(layers: &[&str]) -> MimeDb {
    let layers: Vec<MimeLayer<'_>> =
        layers.iter().map(|g| MimeLayer { globs2: g, ..MimeLayer::default() }).collect();
    MimeDb::from_layers(&layers)
}

#[test]
fn names_map_to_types() {
    let rows = [
        ("notes.txt", Some("text/plain")),
        ("NOTES.TXT", Some("text/plain")),
        ("a.tar.gz", Some("application/x-compressed-tar")),
        ("A.TAR.GZ", Some("application/x-compressed-tar")),
        ("a.gz", Some("application/gzip")),
        ("README", Some("text/x-readme")),
        ("readme", Some("text/x-readme")),
        ("Makefile", Some("text/x-makefile")),
        ("core.123", Some("application/x-core")),
        ("core.x", None),
        ("img_1.raw", Some("image/x-raw")),
        ("img_12.raw", None),
        ("special.txt", Some("text/x-special")),
        ("x.in.txt", Some("text/plain")),
        ("x.in.xt", Some("text/x-prefixed")),
        ("neg_d", Some("text/x-neg")),
        ("neg_b", None),
        ("foo.c", Some("text/x-csrc")),
        ("foo.C", Some("text/x-c++src")),
        ("x.dup", Some("text/x-strong")),
        ("a.bad", None),
        ("nothing", None),
    ];
    let db = db(&[GLOBS]);
    for (file, want) in rows {
        let want = want.map_or(NameGuess::Unknown, |kind| NameGuess::Type(mt(kind)));
        assert_eq!(db.by_name(&name(file)), want, "{file}");
    }
}

#[test]
fn equal_patterns_with_equal_weight_are_ambiguous() {
    let guess = db(&[GLOBS]).by_name(&name("x.amb"));
    assert_eq!(guess, NameGuess::Ambiguous(vec![mt("text/x-amb-a"), mt("text/x-amb-b")]));
}

#[test]
fn nothing_globs_in_a_higher_folder_clears_lower_patterns_of_that_type() {
    let high = "0:text/plain:__NOGLOBS__\n50:text/plain:*.text\n50:image/png:*.png\n";
    let low = "50:text/plain:*.txt\n50:text/plain:*.asc\n50:image/jpeg:*.jpg\n";
    let db = db(&[high, low]);
    let rows = [
        ("a.txt", NameGuess::Unknown),
        ("a.asc", NameGuess::Unknown),
        ("a.text", NameGuess::Type(mt("text/plain"))),
        ("a.png", NameGuess::Type(mt("image/png"))),
        ("a.jpg", NameGuess::Type(mt("image/jpeg"))),
    ];
    for (file, want) in rows {
        assert_eq!(db.by_name(&name(file)), want, "{file}");
    }
}

#[test]
fn nothing_globs_in_a_lower_folder_leaves_higher_patterns() {
    let db = db(&["50:text/plain:*.txt\n", "0:text/plain:__NOGLOBS__\n"]);
    assert_eq!(db.by_name(&name("a.txt")), NameGuess::Type(mt("text/plain")));
}

#[test]
fn a_case_sensitive_pattern_ignores_other_case() {
    let db = db(&["50:text/x-cs:README:cs\n"]);
    assert_eq!(db.by_name(&name("README")), NameGuess::Type(mt("text/x-cs")));
    assert_eq!(db.by_name(&name("readme")), NameGuess::Unknown);
}
