//! Tags: tagging the selection, and the search a tag place runs.

use files_core::{
    ByteSize, Entry, EntryKind, FileName, FolderPath, ItemRef, Location, TagColour, Tags,
};
use files_model::{Browser, Effect, Held, Input, Listing, Mark, Modifier};

use crate::common::{at, dir, file, home, loaded, name};

fn tagged(text: &str, colours: &str) -> Entry {
    file(text).with_tags(Tags::parse(colours))
}

fn select(browser: Browser, names: &[&str]) -> Browser {
    let mut inputs = vec![Input::SelectNone];
    for (i, n) in names.iter().enumerate() {
        let how = if i == 0 {
            Modifier::None
        } else {
            Modifier::Toggle
        };
        inputs.push(Input::Click(name(n), how));
    }
    crate::common::run(browser, inputs).0
}

#[test]
fn tagging_puts_the_tag_on_unless_every_selected_item_has_it() {
    let browser = loaded(vec![tagged("a", "Red"), tagged("b", "Red,Work"), file("c")]);
    let cases: &[(&[&str], Mark, Held)] = &[
        (&["a"], Mark::Remove, Held::Every),
        (&["a", "b"], Mark::Remove, Held::Every),
        (&["a", "c"], Mark::Add, Held::Part),
        (&["c"], Mark::Add, Held::Nothing),
    ];
    for (names, how, held) in cases {
        let browser = select(browser.clone(), names);
        assert_eq!(browser.held(TagColour::Red), *held, "{names:?}");
        let (_, effects) = browser.step(Input::Tag(TagColour::Red), at(0));
        let Some(Effect::Tag {
            items,
            colour,
            how: got,
        }) = effects.first().cloned()
        else {
            panic!("{names:?}: {effects:?}");
        };
        assert_eq!((colour, got), (TagColour::Red, *how), "{names:?}");
        assert_eq!(items.len(), names.len());
    }
}

#[test]
fn tagging_nothing_does_nothing() {
    let browser = loaded(vec![file("a")]);
    let (_, effects) = browser.step(Input::Tag(TagColour::Blue), at(0));
    assert!(effects.is_empty());
}

fn homed() -> Browser {
    let (browser, _) = Browser::new(home());
    browser.with_home(FolderPath::new("/home/p").unwrap())
}

#[test]
fn going_to_a_tag_starts_a_search_from_home_and_watches_nothing() {
    let (browser, effects) = homed().step(Input::Go(Location::Tag(TagColour::Green)), at(0));
    assert_eq!(
        effects,
        vec![
            Effect::Unwatch(home()),
            Effect::Find {
                colour: TagColour::Green,
                within: FolderPath::new("/home/p").unwrap()
            },
            Effect::LoadPrefs(Location::Tag(TagColour::Green)),
        ]
    );
    assert_eq!(browser.listing(), &Listing::Loading);
}

#[test]
fn found_batches_stream_in_and_the_search_ends_loaded() {
    let place = Location::Tag(TagColour::Red);
    let found = |text: &str, folder: &str| {
        Entry::new(name(text), EntryKind::File(ByteSize(1)))
            .found_at(ItemRef::new(FolderPath::new(folder).unwrap(), name(text)))
    };
    let (browser, _) = homed().step(Input::Go(place.clone()), at(0));
    let (browser, _) = browser.step(
        Input::Found(place.clone(), vec![found("n.txt", "/home/p/a")]),
        at(0),
    );
    assert!(matches!(browser.listing(), Listing::Searching(rows) if rows.len() == 1));
    assert_eq!(browser.rows().len(), 1, "rows show while it walks");
    let batch = vec![found("n.txt", "/home/p/b"), found("m.txt", "/home/p/b")];
    let (browser, _) = browser.step(Input::Found(place.clone(), batch), at(0));
    assert_eq!(
        browser.rows().len(),
        3,
        "two files of one name are two rows"
    );
    // A batch for a place the browser has left is ignored.
    let (browser, _) = browser.step(Input::Found(dir("/x"), vec![found("z", "/x")]), at(0));
    assert_eq!(browser.rows().len(), 3);
    let (browser, _) = browser.step(Input::SearchDone(place), at(0));
    assert!(matches!(browser.listing(), Listing::Loaded(rows) if rows.len() == 3));
}

#[test]
fn reloading_a_search_starts_it_over_and_asks_again() {
    let place = Location::Tag(TagColour::Red);
    let (browser, _) = homed().step(Input::Go(place.clone()), at(0));
    let row = Entry::new(name("n"), EntryKind::File(ByteSize(1)))
        .found_at(ItemRef::new(FolderPath::new("/home/p").unwrap(), name("n")));
    let (browser, _) = browser.step(Input::Found(place.clone(), vec![row]), at(0));
    let (browser, effects) =
        browser.step(Input::Changed(place, files_model::Change::Rescan), at(0));
    assert_eq!(browser.listing(), &Listing::Loading);
    assert!(matches!(effects[0], Effect::Find { .. }));
}

#[test]
fn a_found_row_opens_and_trashes_where_it_really_is() {
    let place = Location::Tag(TagColour::Red);
    let real = ItemRef::new(FolderPath::new("/home/p/deep").unwrap(), name("n.txt"));
    let row = Entry::new(name("n.txt"), EntryKind::File(ByteSize(1))).found_at(real.clone());
    let key = row.name.clone();
    let (browser, _) = homed().step(Input::Go(place.clone()), at(0));
    let (browser, _) = browser.step(Input::Found(place, vec![row]), at(0));
    let (browser, _) = browser.step(Input::Click(key, Modifier::None), at(0));
    let (browser, effects) = browser.step(Input::Open, at(0));
    let Some(Effect::OpenFiles(targets)) = effects.first() else {
        panic!("{effects:?}");
    };
    assert_eq!(targets[0].path(), real.path());
    let (_, effects) = browser.step(Input::MoveToTrash, at(0));
    assert!(matches!(&effects[0], Effect::Run(_)));
}

#[test]
fn the_shown_name_of_a_found_row_is_its_own() {
    let real = ItemRef::new(
        FolderPath::new("/home/p/deep").unwrap(),
        FileName::new("n.txt").unwrap(),
    );
    let row = Entry::new(name("x"), EntryKind::File(ByteSize(1))).found_at(real);
    assert_eq!(row.shown_name().display(), "n.txt");
}
