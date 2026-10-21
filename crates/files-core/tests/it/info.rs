//! Get Info's facts and the words preferences are stored in.

use files_core::{
    ByteSize, EntryKind, FileName, FolderPath, Info, ItemRef, LinkTo, SortKey, SortOrder, Special,
    ViewMode,
};

fn item(name: &str) -> ItemRef {
    ItemRef::new(
        FolderPath::new("/home/p").unwrap(),
        FileName::new(name).unwrap(),
    )
}

#[test]
fn only_a_file_or_a_link_to_one_has_a_size() {
    let cases = [
        ("file", EntryKind::File(ByteSize(5)), Some(ByteSize(5))),
        (
            "link to a file",
            EntryKind::Link(LinkTo::File(ByteSize(7))),
            Some(ByteSize(7)),
        ),
        ("folder", EntryKind::Folder, None),
        ("link to a folder", EntryKind::Link(LinkTo::Folder), None),
        ("broken link", EntryKind::Link(LinkTo::Broken), None),
        ("pipe", EntryKind::Special(Special::Fifo), None),
    ];
    for (label, kind, want) in cases {
        assert_eq!(Info::new(item("x"), kind).size(), want, "{label}");
    }
}

#[test]
fn stored_words_name_every_view_key_and_order_and_nothing_else() {
    for view in [ViewMode::List, ViewMode::Icons] {
        assert_eq!(ViewMode::from_slug(view.slug()), Some(view));
    }
    for key in [
        SortKey::Name,
        SortKey::Modified,
        SortKey::Size,
        SortKey::Kind,
    ] {
        assert_eq!(SortKey::from_slug(key.slug()), Some(key));
    }
    for order in [SortOrder::Ascending, SortOrder::Descending] {
        assert_eq!(SortOrder::from_slug(order.slug()), Some(order));
    }
    assert_eq!(ViewMode::from_slug("columns"), None);
    assert_eq!(SortKey::from_slug(""), None);
    assert_eq!(SortOrder::from_slug("Ascending"), None);
}
