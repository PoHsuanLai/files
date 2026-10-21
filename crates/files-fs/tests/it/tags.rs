//! Tags: the real tagger in a scratch folder, the fake, the tagged lister and the walk.

use std::fs;
use std::sync::Arc;

use files_core::{ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Tag, TagColour, Tags};
use files_fs::{
    ATTRIBUTE, DiskLister, FakeLister, FakeTagger, Flow, Found, Lister, TagWriteError, Tagged,
    Tagger, WALK_CAP, Walked, XattrTagger, find_tagged,
};
use tempfile::TempDir;

fn folder(s: &str) -> FolderPath {
    FolderPath::new(s).unwrap()
}

fn item(dir: &str, name: &str) -> ItemRef {
    ItemRef::new(folder(dir), FileName::new(name).unwrap())
}

fn file(name: &str) -> Entry {
    Entry::new(FileName::new(name).unwrap(), EntryKind::File(ByteSize(1)))
}

fn dir_entry(name: &str) -> Entry {
    Entry::new(FileName::new(name).unwrap(), EntryKind::Folder)
}

/// A scratch file, when the scratch disk keeps user attributes; else a printed note and `None`.
fn scratch_file() -> Option<(TempDir, ItemRef)> {
    let dir = TempDir::new().unwrap();
    let path = dir.path().join("a.txt");
    fs::write(&path, "x").unwrap();
    match xattr::set(&path, "user.files-probe", b"1") {
        Ok(()) => {
            let at = ItemRef::from_path(&path).unwrap();
            Some((dir, at))
        }
        Err(err) => {
            eprintln!("skipped: the scratch disk keeps no user attributes ({err})");
            None
        }
    }
}

#[test]
fn the_real_tagger_round_trips_the_attribute_and_removes_it_when_empty() {
    let Some((_dir, at)) = scratch_file() else {
        return;
    };
    assert!(XattrTagger.tags(&at).is_empty());
    let tags = Tags::none()
        .with(Tag::of(TagColour::Red))
        .with(Tag::new("Work").unwrap());
    XattrTagger.set(&at, &tags).unwrap();
    assert_eq!(XattrTagger.tags(&at), tags);
    let raw = xattr::get(at.path(), ATTRIBUTE).unwrap().unwrap();
    assert_eq!(raw, b"Red,Work");
    XattrTagger.set(&at, &Tags::none()).unwrap();
    assert!(xattr::get(at.path(), ATTRIBUTE).unwrap().is_none());
    // Removing what is not there is not an error.
    XattrTagger.set(&at, &Tags::none()).unwrap();
}

#[test]
fn the_real_tagger_says_a_missing_file_is_not_found() {
    let dir = TempDir::new().unwrap();
    let gone = ItemRef::from_path(&dir.path().join("gone")).unwrap();
    let tags = Tags::none().with(Tag::of(TagColour::Blue));
    let err = XattrTagger.set(&gone, &tags).unwrap_err();
    assert_eq!(err, TagWriteError::NotFound);
    assert!(XattrTagger.tags(&gone).is_empty());
}

#[test]
fn an_error_says_in_plain_words_what_went_wrong() {
    let cases = [
        (
            TagWriteError::Unsupported,
            "Tags can\u{2019}t be added on this disk.",
        ),
        (TagWriteError::NotFound, "That file can\u{2019}t be found."),
    ];
    for (error, words) in cases {
        assert_eq!(error.to_string(), words);
    }
}

#[test]
fn the_fake_remembers_and_can_refuse() {
    let at = item("/p", "a");
    let fake = FakeTagger::new();
    let tags = Tags::parse("Green");
    fake.set(&at, &tags).unwrap();
    assert_eq!(fake.tags(&at), tags);
    let refusing = FakeTagger::refusing(TagWriteError::Unsupported);
    assert_eq!(refusing.set(&at, &tags), Err(TagWriteError::Unsupported));
    assert!(refusing.tags(&at).is_empty());
}

#[test]
fn a_tagged_lister_gives_every_entry_the_tags_of_the_tagger() {
    let lister = FakeLister::new();
    lister.set(folder("/p"), Ok(vec![file("a"), file("b")]));
    let tagger = FakeTagger::new();
    tagger.put(item("/p", "b"), Tags::parse("Orange"));
    let tagged = Tagged::new(Arc::new(lister), Arc::new(tagger));
    let mut entries = tagged.list(&folder("/p")).unwrap();
    entries.sort_by(|x, y| x.name.cmp(&y.name));
    let marks: Vec<_> = entries
        .iter()
        .map(|e| e.tags.has(TagColour::Orange))
        .collect();
    assert_eq!(marks, vec![false, true]);
    let one = tagged
        .entry(&folder("/p"), &FileName::new("b").unwrap())
        .unwrap();
    assert!(one.tags.has(TagColour::Orange));
}

#[test]
fn a_disk_lister_under_a_tagger_lists_what_is_tagged() {
    let dir = TempDir::new().unwrap();
    fs::write(dir.path().join("a.txt"), "x").unwrap();
    let here = FolderPath::new(dir.path()).unwrap();
    let at = ItemRef::new(here.clone(), FileName::new("a.txt").unwrap());
    let tagger = Arc::new(FakeTagger::new());
    tagger.put(at, Tags::parse("Purple"));
    let tagged = Tagged::new(Arc::new(DiskLister), tagger);
    let entries = tagged.list(&here).unwrap();
    assert!(entries[0].tags.has(TagColour::Purple));
}

/// A tree: /r holds a (tagged Red), sub/, .hidden/; sub holds b (Red) and c (Blue); .hidden holds d (Red).
fn tree() -> (Arc<FakeLister>, Arc<FakeTagger>) {
    let lister = Arc::new(FakeLister::new());
    lister.set(
        folder("/r"),
        Ok(vec![file("a"), dir_entry("sub"), dir_entry(".hidden")]),
    );
    lister.set(folder("/r/sub"), Ok(vec![file("b"), file("c")]));
    lister.set(folder("/r/.hidden"), Ok(vec![file("d")]));
    let tagger = Arc::new(FakeTagger::new());
    tagger.put(item("/r", "a"), Tags::parse("Red"));
    tagger.put(item("/r/sub", "b"), Tags::parse("Work,Red"));
    tagger.put(item("/r/sub", "c"), Tags::parse("Blue"));
    tagger.put(item("/r/.hidden", "d"), Tags::parse("Red"));
    (lister, tagger)
}

fn walk(cap: usize, flow: Flow) -> (Vec<String>, Walked) {
    let (lister, tagger) = tree();
    let tagged = Tagged::new(lister, tagger);
    let mut names = Vec::new();
    let ended = find_tagged(
        &tagged,
        &folder("/r"),
        TagColour::Red,
        cap,
        &mut |batch: Vec<Found>| {
            names.extend(batch.iter().map(|f| f.item.path().display().to_string()));
            flow
        },
    );
    names.sort();
    (names, ended)
}

#[test]
fn the_walk_finds_the_colour_in_nested_folders_and_skips_hidden_ones() {
    let (names, ended) = walk(WALK_CAP, Flow::Go);
    assert_eq!(names, vec!["/r/a", "/r/sub/b"]);
    assert_eq!(ended, Walked::Finished);
}

#[test]
fn the_walk_stops_at_its_cap_and_when_told_to() {
    // Three entries in /r are looked at, then the cap.
    let (names, ended) = walk(3, Flow::Go);
    assert_eq!(names, vec!["/r/a"]);
    assert_eq!(ended, Walked::Capped);
    let (_, ended) = walk(WALK_CAP, Flow::Stop);
    assert_eq!(ended, Walked::Stopped);
}
