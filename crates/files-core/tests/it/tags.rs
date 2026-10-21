//! Tags: names, the seven colours, and the attribute they are kept in.

use files_core::{
    ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Tag, TagColour, TagError, Tags,
};

#[test]
fn a_tag_is_a_trimmed_name_without_commas_or_control_characters() {
    let cases: &[(&str, Result<&str, TagError>)] = &[
        ("Red", Ok("Red")),
        ("  Work  ", Ok("Work")),
        ("", Err(TagError::Empty)),
        ("   ", Err(TagError::Empty)),
        ("a,b", Err(TagError::Comma)),
        ("a\nb", Err(TagError::Control)),
    ];
    for (text, want) in cases {
        let got = Tag::new(text);
        assert_eq!(
            got.as_ref().map(Tag::name),
            want.as_ref().map(|s| *s),
            "{text:?}"
        );
    }
}

#[test]
fn the_seven_colours_are_tags_named_like_them() {
    for colour in TagColour::ALL {
        assert_eq!(Tag::of(colour).colour(), Some(colour));
        assert_eq!(
            TagColour::named(&colour.label().to_lowercase()),
            Some(colour)
        );
    }
    assert_eq!(Tag::new("Work").unwrap().colour(), None);
    assert_eq!(TagColour::ALL.len(), 7);
}

#[test]
fn the_attribute_is_a_comma_separated_list_read_leniently() {
    let cases: &[(&str, &str)] = &[
        ("Red,Work", "Red,Work"),
        (" Red , Work ", "Red,Work"),
        ("Red,,Red,", "Red"),
        ("", ""),
        ("Work,Green", "Work,Green"),
    ];
    for (text, want) in cases {
        assert_eq!(Tags::parse(text).to_attribute(), *want, "{text:?}");
    }
}

#[test]
fn a_colour_is_toggled_without_touching_the_other_tags() {
    let tags = Tags::parse("Work,Red");
    let off = tags.clone().toggled(TagColour::Red);
    assert_eq!(off.to_attribute(), "Work");
    let on = off.toggled(TagColour::Blue);
    assert_eq!(on.to_attribute(), "Work,Blue");
    assert!(on.has(TagColour::Blue) && !on.has(TagColour::Red));
    assert_eq!(on.colours().collect::<Vec<_>>(), vec![TagColour::Blue]);
}

#[test]
fn a_found_row_is_keyed_by_its_whole_path_and_shows_its_own_name() {
    let item = |folder: &str| {
        ItemRef::new(
            FolderPath::new(folder).unwrap(),
            FileName::new("notes.txt").unwrap(),
        )
    };
    let row = |folder: &str| {
        Entry::new(
            FileName::new("notes.txt").unwrap(),
            EntryKind::File(ByteSize(1)),
        )
        .found_at(item(folder))
    };
    let (a, b) = (row("/home/p/a"), row("/home/p/b"));
    assert_ne!(a.name, b.name);
    assert_eq!(a.shown_name().display(), "notes.txt");
    assert_eq!(a.found, Some(item("/home/p/a")));
    assert_eq!(a.name, row("/home/p/a").name);
}
