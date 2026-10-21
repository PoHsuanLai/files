//! The edge with several tabs: an answer goes back to the tab that asked, a watch belongs to one
//! tab, and tags are written and searched for.

use std::sync::Arc;

use files_core::{Entry, ItemRef, Location, TagColour, Tags};
use files_fs::{FolderEvent, ListError, TagWriteError, Tagger};
use files_model::{Change, Effect, Input, Mark, TabId, Tabbed};
use files_ui::NoVolumes;
use files_ui::edge::Heard;

use super::edge::{file, folder, name, next, rig, rig_with};

/// The tab ids a window hands out: the first, and the ones a New Tab opens after it.
fn tabs() -> (TabId, TabId) {
    let (tabs, _) = files_model::Tabs::new(Location::Trash);
    let (tabs, _) = tabs.step(Input::NewTab, files_model::Now(std::time::Duration::ZERO));
    (TabId::FIRST, tabs.front_id())
}

fn item(dir: &str, text: &str) -> ItemRef {
    ItemRef::new(folder(dir), name(text))
}

#[test]
fn a_listing_goes_back_to_the_tab_that_asked_whichever_is_in_front() {
    let (first, second) = tabs();
    let mut rig = rig();
    rig.lister.set(folder("/p"), Ok(vec![file("a")]));
    rig.lister.set(folder("/q"), Ok(vec![file("b")]));
    let (p, q) = (
        Location::Folder(folder("/p")),
        Location::Folder(folder("/q")),
    );
    rig.edge.run(vec![
        Tabbed::new(second, Effect::List(q.clone())),
        Tabbed::new(first, Effect::List(p.clone())),
    ]);
    let mut heard = vec![next(&mut rig.inbox), next(&mut rig.inbox)];
    heard.sort_by_key(|h| format!("{h:?}"));
    let want = [
        Heard::Tab(first, Input::Listed(p, Ok(vec![file("a")]))),
        Heard::Tab(second, Input::Listed(q, Ok(vec![file("b")]))),
    ];
    for want in want {
        assert!(heard.contains(&want), "{want:?} is not in {heard:?}");
    }
}

#[test]
fn two_tabs_on_one_folder_each_watch_it_and_each_stop_alone() {
    let (first, second) = tabs();
    let mut rig = rig();
    let here = folder("/p");
    let place = Location::Folder(here.clone());
    rig.edge.run(vec![
        Tabbed::new(first, Effect::Watch(place.clone())),
        Tabbed::new(second, Effect::Watch(place.clone())),
    ]);
    assert_eq!(rig.watcher.watched(), vec![here.clone(), here.clone()]);
    rig.lister.set(here.clone(), Ok(vec![file("a")]));
    rig.watcher.push(&here, FolderEvent::Changed(name("a")));
    // Both hear the change, each as its own.
    let heard = [next(&mut rig.inbox), next(&mut rig.inbox)];
    for tab in [first, second] {
        let want = Heard::Tab(
            tab,
            Input::Changed(place.clone(), Change::Updated(file("a"))),
        );
        assert!(heard.contains(&want), "{want:?} is not in {heard:?}");
    }
    rig.edge
        .run(vec![Tabbed::new(first, Effect::Unwatch(place))]);
    assert_eq!(rig.watcher.watched(), vec![here]);
}

#[test]
fn the_effect_that_closes_the_window_is_heard_as_such() {
    let mut rig = rig();
    rig.edge.run_in(TabId::FIRST, vec![Effect::CloseWindow]);
    assert_eq!(next(&mut rig.inbox), Heard::CloseWindow);
}

#[test]
fn a_tag_is_put_on_and_taken_off_and_the_places_are_rescanned() {
    let mut rig = rig();
    let at = item("/p", "a.txt");
    let put = |how| Effect::Tag {
        items: vec![at.clone()],
        colour: TagColour::Red,
        how,
    };
    rig.edge.run_in(TabId::FIRST, vec![put(Mark::Add)]);
    let rescans = [next(&mut rig.inbox), next(&mut rig.inbox)];
    assert!(rig.tagger.tags(&at).has(TagColour::Red));
    for place in [
        Location::Folder(folder("/p")),
        Location::Tag(TagColour::Red),
    ] {
        let want = Heard::All(Input::Changed(place, Change::Rescan));
        assert!(rescans.contains(&want), "{want:?} is not in {rescans:?}");
    }
    rig.edge.run_in(TabId::FIRST, vec![put(Mark::Remove)]);
    let _ = [next(&mut rig.inbox), next(&mut rig.inbox)];
    assert!(rig.tagger.tags(&at).is_empty());
}

#[test]
fn a_tag_leaves_the_other_tags_of_the_file_alone() {
    let mut rig = rig();
    let at = item("/p", "a.txt");
    rig.tagger.put(at.clone(), Tags::parse("Work,Red"));
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Tag {
            items: vec![at.clone()],
            colour: TagColour::Red,
            how: Mark::Remove,
        }],
    );
    let _ = [next(&mut rig.inbox), next(&mut rig.inbox)];
    assert_eq!(rig.tagger.tags(&at).to_attribute(), "Work");
}

#[test]
fn a_disk_that_keeps_no_tags_says_so_in_plain_words() {
    let tagger = Arc::new(files_fs::FakeTagger::refusing(TagWriteError::Unsupported));
    let mut rig = rig_with(Arc::new(NoVolumes), tagger);
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Tag {
            items: vec![item("/p", "a"), item("/p", "b")],
            colour: TagColour::Blue,
            how: Mark::Add,
        }],
    );
    // Said once, at the first file, and the rest is not tried.
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Told("Tags can\u{2019}t be added on this disk.".to_owned())
    );
}

/// A tree under /home/p: a (Red), sub/b (Red, Work), sub/c (Blue), .cache/d (Red).
fn tree(rig: &super::edge::Rig) {
    let dir = |text: &str| Entry::new(name(text), files_core::EntryKind::Folder);
    rig.lister.set(
        folder("/home/p"),
        Ok(vec![file("a"), dir("sub"), dir(".cache")]),
    );
    rig.lister
        .set(folder("/home/p/sub"), Ok(vec![file("b"), file("c")]));
    rig.lister
        .set(folder("/home/p/.cache"), Ok(vec![file("d")]));
    rig.tagger.put(item("/home/p", "a"), Tags::parse("Red"));
    rig.tagger
        .put(item("/home/p/sub", "b"), Tags::parse("Work,Red"));
    rig.tagger
        .put(item("/home/p/sub", "c"), Tags::parse("Blue"));
    rig.tagger
        .put(item("/home/p/.cache", "d"), Tags::parse("Red"));
}

/// What the search of `colour` sent to `tab`, up to its end: the paths found, then true.
fn found(rig: &mut super::edge::Rig, tab: TabId, colour: TagColour) -> Vec<String> {
    let place = Location::Tag(colour);
    let mut paths = Vec::new();
    loop {
        match next(&mut rig.inbox) {
            Heard::Tab(who, Input::Found(at, rows)) => {
                assert_eq!((who, at), (tab, place.clone()));
                paths.extend(
                    rows.iter()
                        .map(|row| row.found.as_ref().unwrap().path().display().to_string()),
                );
            }
            Heard::Tab(who, Input::SearchDone(at)) => {
                assert_eq!((who, at), (tab, place));
                paths.sort();
                return paths;
            }
            other => panic!("not part of a search: {other:?}"),
        }
    }
}

#[test]
fn a_tag_place_streams_the_files_with_its_colour_to_the_tab_that_asked() {
    let (_, second) = tabs();
    let mut rig = rig();
    tree(&rig);
    let find = Effect::Find {
        colour: TagColour::Red,
        within: folder("/home/p"),
    };
    rig.edge.run(vec![Tabbed::new(second, find)]);
    let paths = found(&mut rig, second, TagColour::Red);
    assert_eq!(paths, vec!["/home/p/a", "/home/p/sub/b"]);
}

#[test]
fn a_tag_search_of_a_folder_that_cannot_be_read_ends_empty() {
    let mut rig = rig();
    rig.lister
        .set(folder("/home/p"), Err(ListError::NoPermission));
    rig.edge.run_in(
        TabId::FIRST,
        vec![Effect::Find {
            colour: TagColour::Green,
            within: folder("/home/p"),
        }],
    );
    assert!(found(&mut rig, TabId::FIRST, TagColour::Green).is_empty());
}
