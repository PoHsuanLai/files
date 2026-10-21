//! The remembered folder views: the book, its text, and the file store.

use std::fs;

use files_core::{FolderPath, FolderPrefs, Sort, SortKey, SortOrder, ViewMode};
use files_fs::{FakePrefs, KEPT, PrefBook, PrefsError, PrefsStore, TomlPrefs};
use tempfile::TempDir;

fn folder(path: &str) -> FolderPath {
    FolderPath::new(path).unwrap()
}

fn icons_by_size() -> FolderPrefs {
    let mut prefs = FolderPrefs::default();
    prefs.view = ViewMode::Icons;
    prefs.sort = Sort {
        key: SortKey::Size,
        order: SortOrder::Descending,
    };
    prefs
}

#[test]
fn the_book_remembers_choices_and_forgets_a_return_to_the_defaults() {
    let mut book = PrefBook::default();
    book.set(folder("/a"), icons_by_size());
    assert_eq!(book.get(&folder("/a")), Some(icons_by_size()));
    assert_eq!(book.get(&folder("/b")), None);
    book.set(folder("/a"), FolderPrefs::default());
    assert!(book.is_empty());
}

#[test]
fn the_book_round_trips_through_its_text() {
    let mut book = PrefBook::default();
    book.set(folder("/a"), icons_by_size());
    book.set(folder("/with \"quotes\" and \\"), icons_by_size());
    let back = PrefBook::parse(&book.render()).unwrap();
    assert_eq!(back.get(&folder("/a")), Some(icons_by_size()));
    assert_eq!(
        back.get(&folder("/with \"quotes\" and \\")),
        Some(icons_by_size())
    );
    assert_eq!(back.len(), 2);
}

#[test]
fn text_is_read_leniently() {
    // label, text, folder asked, what it should give
    let cases: [(&str, &str, &str, Option<FolderPrefs>); 5] = [
        (
            "an unknown word costs only itself",
            "[[folder]]\npath = \"/a\"\nview = \"columns\"\nkey = \"size\"\n",
            "/a",
            Some({
                let mut prefs = FolderPrefs::default();
                prefs.sort.key = SortKey::Size;
                prefs
            }),
        ),
        (
            "a relative path is skipped",
            "[[folder]]\npath = \"a\"\nview = \"icons\"\n",
            "/a",
            None,
        ),
        (
            "a table with no path is skipped",
            "[[folder]]\nview = \"icons\"\n",
            "/a",
            None,
        ),
        ("an empty file is an empty book", "", "/a", None),
        (
            "another table is ignored",
            "[other]\nx = 1\n\n[[folder]]\npath = \"/a\"\nview = \"icons\"\n",
            "/a",
            Some({
                let mut prefs = FolderPrefs::default();
                prefs.view = ViewMode::Icons;
                prefs
            }),
        ),
    ];
    for (label, text, asked, want) in cases {
        let got = PrefBook::parse(text).unwrap().get(&folder(asked));
        assert_eq!(got, want, "{label}");
    }
    assert!(PrefBook::parse("not = [valid").is_none());
}

#[test]
fn the_book_keeps_the_newest_changes_and_no_more() {
    let mut book = PrefBook::default();
    for n in 0..KEPT + 3 {
        book.set(folder(&format!("/f{n}")), icons_by_size());
    }
    assert_eq!(book.len(), KEPT);
    assert_eq!(book.get(&folder("/f0")), None);
    assert_eq!(book.get(&folder("/f2")), None);
    assert!(book.get(&folder("/f3")).is_some());
    // Changing an old folder again makes it new.
    book.set(folder("/f3"), icons_by_size());
    book.set(folder("/new"), icons_by_size());
    assert!(book.get(&folder("/f3")).is_some());
    assert_eq!(book.get(&folder("/f4")), None);
    // The order survives the file.
    let back = PrefBook::parse(&book.render()).unwrap();
    back.get(&folder("/new")).unwrap();
    assert_eq!(back.len(), KEPT);
}

#[test]
fn the_file_store_keeps_choices_between_runs() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("state").join("folders.toml");
    let first = TomlPrefs::new(file.clone());
    assert_eq!(first.load(&folder("/a")), FolderPrefs::default());
    first.save(&folder("/a"), icons_by_size()).unwrap();
    let second = TomlPrefs::new(file);
    assert_eq!(second.load(&folder("/a")), icons_by_size());
    assert_eq!(second.load(&folder("/b")), FolderPrefs::default());
}

#[test]
fn a_file_that_cannot_be_understood_is_left_alone() {
    let dir = TempDir::new().unwrap();
    let file = dir.path().join("folders.toml");
    fs::write(&file, "this is [not toml").unwrap();
    let store = TomlPrefs::new(file.clone());
    assert_eq!(store.load(&folder("/a")), FolderPrefs::default());
    assert!(matches!(
        store.save(&folder("/a"), icons_by_size()),
        Err(PrefsError::Unusable)
    ));
    assert_eq!(fs::read_to_string(file).unwrap(), "this is [not toml");
}

#[test]
fn the_fake_store_stands_for_a_relaunch() {
    let store = FakePrefs::new();
    store.save(&folder("/a"), icons_by_size()).unwrap();
    assert_eq!(store.load(&folder("/a")), icons_by_size());
}
