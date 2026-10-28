//! Going places: Go, Back, Forward, Enclosing and the effects each asks for.

use files_model::{Browser, Effect, Input};

use crate::common::*;

#[test]
fn a_new_browser_asks_to_list_watch_and_load_prefs() {
    let (browser, effects) = Browser::new(home());
    assert_eq!(
        effects,
        vec![
            Effect::List(home()),
            Effect::Watch(home()),
            Effect::LoadPrefs(home())
        ]
    );
    assert_eq!(browser.location(), &home());
    assert!(!browser.can_go_back());
}

#[test]
fn going_somewhere_unwatches_the_old_place_and_loads_the_new() {
    let (browser, _) = Browser::new(home());
    let docs = dir("/home/p/Documents");
    let (browser, effects) = browser.step(Input::Go(docs.clone()), at(0));
    assert_eq!(
        effects,
        vec![
            Effect::Unwatch(home()),
            Effect::List(docs.clone()),
            Effect::Watch(docs.clone()),
            Effect::LoadPrefs(docs.clone()),
        ]
    );
    assert_eq!(browser.location(), &docs);
    assert!(browser.can_go_back());
    assert!(!browser.can_go_forward());
}

#[test]
fn back_forward_and_enclosing_walk_the_history() {
    // (input, where the browser ends up)
    let steps: Vec<(Input, &str)> = vec![
        (Input::Go(dir("/home/p/a")), "/home/p/a"),
        (Input::Go(dir("/home/p/a/b")), "/home/p/a/b"),
        (Input::Back, "/home/p/a"),
        (Input::Back, "/home/p"),
        (Input::Back, "/home/p"),
        (Input::Forward, "/home/p/a"),
        (Input::Enclosing, "/home/p"),
        (Input::Back, "/home/p/a"),
        (Input::Forward, "/home/p"),
    ];
    let (mut browser, _) = Browser::new(home());
    for (input, want) in steps {
        browser = browser.step(input.clone(), at(0)).0;
        assert_eq!(path_of(browser.location()), want, "{input:?}");
    }
}

#[test]
fn going_somewhere_new_forgets_what_lay_ahead() {
    let (browser, _) = run(
        loaded(vec![]),
        vec![Input::Go(dir("/a")), Input::Back, Input::Go(dir("/b"))],
    );
    assert!(!browser.can_go_forward());
    let (browser, effects) = browser.step(Input::Forward, at(0));
    assert_eq!(path_of(browser.location()), "/b");
    assert!(effects.is_empty());
}

#[test]
fn back_and_enclosing_stop_at_their_ends() {
    let (browser, _) = Browser::new(home());
    let (after, effects) = browser.clone().step(Input::Back, at(0));
    assert_eq!((after, effects), (browser, vec![]), "nowhere behind");
    let (browser, _) = Browser::new(dir("/"));
    assert!(!browser.can_go_enclosing());
    let (after, effects) = browser.clone().step(Input::Enclosing, at(0));
    assert_eq!(after, browser);
    assert!(effects.is_empty());
    let (browser, _) = Browser::new(home());
    assert!(browser.can_go_enclosing());
}

#[test]
fn going_to_the_same_place_reads_it_again_without_a_history_entry() {
    let (browser, effects) = loaded(vec![file("a")]).step(Input::Go(home()), at(0));
    assert_eq!(effects, vec![Effect::List(home())]);
    assert!(!browser.can_go_back());
    assert_eq!(shown(&browser), vec!["a"]);
}

#[test]
fn arriving_forgets_the_old_listing_and_selection_but_keeps_the_hidden_choice() {
    let browser = loaded(vec![file("a"), file(".h")]);
    let (browser, _) = run(
        browser,
        vec![
            Input::ToggleHidden,
            Input::Click(name("a"), files_model::Modifier::None),
        ],
    );
    let (browser, _) = browser.step(Input::Go(dir("/x")), at(0));
    assert_eq!(browser.listing(), &files_model::Listing::Loading);
    assert!(browser.rows().is_empty());
    assert!(browser.selection().is_empty());
    assert!(browser.show_hidden());
}
