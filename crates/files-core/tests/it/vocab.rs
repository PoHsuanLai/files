//! The vocabulary's own rules: what is a folder location, a hidden entry.

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, PathError};

#[test]
fn a_folder_location_is_absolute_and_walks_up_and_down() {
    assert_eq!(FolderPath::new("Documents"), Err(PathError::Relative));
    let home = FolderPath::new("/home/p").unwrap();
    let docs = home.join(&FileName::new("Documents").unwrap());
    assert_eq!(docs.as_path(), std::path::Path::new("/home/p/Documents"));
    assert_eq!(docs.parent(), Some(home));
    assert_eq!(FolderPath::new("/").unwrap().parent(), None);
}

#[test]
fn an_entry_is_hidden_when_its_name_starts_with_a_dot() {
    let entry = |name: &str| Entry::new(FileName::new(name).unwrap(), EntryKind::File(ByteSize(1)));
    assert!(entry(".bashrc").is_hidden());
    assert!(!entry("notes.txt").is_hidden());
    assert!(!entry("a.hidden-not").is_hidden());
}
