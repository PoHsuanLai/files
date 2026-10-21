//! Plans of work: naming a second item, progress, and what a plan is about.

use files_core::{
    ByteSize, FileName, FolderPath, ItemError, ItemRef, OpKind, OpPlan, Progress, Split, Transfer,
    keep_both,
};

fn name(text: &str) -> FileName {
    FileName::new(text).unwrap()
}

fn item(folder: &str, text: &str) -> ItemRef {
    ItemRef::new(FolderPath::new(folder).unwrap(), name(text))
}

fn holds(names: &[&str], name: &FileName) -> bool {
    names.iter().any(|held| *held == &*name.display())
}

#[test]
fn a_taken_name_gets_the_next_free_number() {
    // (name, split, taken names, result)
    let table: &[(&str, Split, &[&str], &str)] = &[
        ("report.pdf", Split::Extension, &[], "report.pdf"),
        (
            "report.pdf",
            Split::Extension,
            &["report.pdf"],
            "report 2.pdf",
        ),
        (
            "report.pdf",
            Split::Extension,
            &["report.pdf", "report 2.pdf"],
            "report 3.pdf",
        ),
        ("notes", Split::Extension, &["notes"], "notes 2"),
        (".bashrc", Split::Extension, &[".bashrc"], ".bashrc 2"),
        ("v1.2", Split::Whole, &["v1.2"], "v1.2 2"),
        ("v1.2", Split::Extension, &["v1.2"], "v1 2.2"),
        (
            "untitled folder",
            Split::Whole,
            &["untitled folder", "untitled folder 2"],
            "untitled folder 3",
        ),
    ];
    for (given, split, names, want) in table {
        let got = keep_both(&name(given), *split, |n| holds(names, n)).unwrap();
        assert_eq!(got.display(), *want, "{given} {names:?}");
    }
}

#[test]
fn a_number_that_makes_the_name_too_long_is_refused() {
    let long = name(&"a".repeat(255));
    assert!(keep_both(&long, Split::Whole, |n| *n == long).is_err());
}

#[test]
fn an_item_is_a_folder_and_a_name() {
    let got = ItemRef::from_path(std::path::Path::new("/home/p/a.txt")).unwrap();
    assert_eq!(got, item("/home/p", "a.txt"));
    assert_eq!(got.path(), std::path::Path::new("/home/p/a.txt"));
    assert_eq!(got.as_folder(), FolderPath::new("/home/p/a.txt").unwrap());
    assert!(matches!(
        ItemRef::from_path(std::path::Path::new("a.txt")),
        Err(ItemError::Path(_))
    ));
    assert_eq!(
        ItemRef::from_path(std::path::Path::new("/")),
        Err(ItemError::NoName)
    );
}

#[test]
fn progress_counts_bytes_when_there_are_some_and_items_otherwise() {
    let bytes = Progress::start(4, ByteSize(200)).wrote(50);
    assert_eq!(bytes.permille(), 250);
    let items = Progress::start(4, ByteSize(0)).finished_one();
    assert_eq!(items.permille(), 250);
    assert_eq!(Progress::start(0, ByteSize(0)).permille(), 1000);
    assert_eq!(Progress::start(1, ByteSize(10)).wrote(99).permille(), 1000);
}

#[test]
fn a_plan_says_what_it_is_about() {
    let dest = FolderPath::new("/d").unwrap();
    let moved = OpPlan::Drag(Transfer::new(vec![item("/a", "x"), item("/a", "y")], dest));
    let summary = moved.summary();
    assert_eq!(
        (summary.kind, summary.count, summary.first),
        (OpKind::Move, 2, Some(name("x")))
    );
    assert_eq!(OpPlan::EmptyTrash.summary().kind, OpKind::Erase);
}
