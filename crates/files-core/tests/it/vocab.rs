//! The vocabulary's own rules: what is a type, a folder location, a hidden entry.

use files_core::{
    ByteSize, Entry, EntryKind, FileName, FolderPath, MimeError, MimeType, PathError, Visibility,
};

#[test]
fn a_mime_type_is_one_lower_cased_pair() {
    let cases: &[(&str, Result<&str, MimeError>)] = &[
        ("image/PNG", Ok("image/png")),
        ("inode/directory", Ok(MimeType::FOLDER)),
        ("image", Err(MimeError::Shape)),
        ("image/", Err(MimeError::Shape)),
        ("/png", Err(MimeError::Shape)),
        ("a/b/c", Err(MimeError::Shape)),
        ("text/plain; charset=utf-8", Err(MimeError::Shape)),
    ];
    for (text, want) in cases {
        let got = MimeType::new(text);
        assert_eq!(
            got.as_ref().map(MimeType::as_str),
            want.as_ref().map(|s| *s),
            "{text}"
        );
    }
    assert_eq!(
        MimeType::new("image/png").map(|m| m.major().to_owned()),
        Ok("image".to_owned())
    );
}

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
    assert_eq!(entry(".bashrc").visibility(), Visibility::Hidden);
    assert_eq!(entry("notes.txt").visibility(), Visibility::Shown);
    assert_eq!(entry("a.hidden-not").visibility(), Visibility::Shown);
}
