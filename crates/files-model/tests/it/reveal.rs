//! Revealing entries: another program asked to show them in their folder.

use files_core::FolderPath;
use files_model::{Availability, Browser, Change, Effect, Input, Modifier};

use crate::common::*;

fn reveal(path: &str, names: &[&str]) -> Input {
    Input::Reveal {
        folder: FolderPath::new(path).unwrap(),
        names: names.iter().map(|n| name(n)).collect(),
    }
}

fn entries() -> Vec<files_core::Entry> {
    vec![file("a.txt"), file("b.txt"), file("c.txt"), folder("dir")]
}

#[test]
fn names_are_selected_in_the_folder_already_shown_and_the_first_is_the_cursor() {
    // (asked, selected in name order, cursor)
    let table: Vec<(&[&str], Vec<&str>, Option<&str>)> = vec![
        (&["b.txt"], vec!["b.txt"], Some("b.txt")),
        (&["c.txt", "a.txt"], vec!["a.txt", "c.txt"], Some("c.txt")),
        (&["a.txt", "gone.txt"], vec!["a.txt"], Some("a.txt")),
        (&["gone.txt"], vec![], None),
        (&[], vec![], None),
    ];
    for (asked, selection, cursor) in table {
        let (browser, effects) = loaded(entries()).step(reveal("/home/p", asked), at(0));
        assert_eq!(selected(&browser), selection, "{asked:?}");
        assert_eq!(
            browser.cursor().map(|n| n.display().into_owned()),
            cursor.map(str::to_owned),
            "{asked:?}"
        );
        // Same place: no visit, but the folder is read again.
        assert_eq!(
            browser.can_go_back(),
            Availability::Unavailable,
            "{asked:?}"
        );
        assert_eq!(effects, vec![Effect::List(home())], "{asked:?}");
    }
}

#[test]
fn a_missing_name_leaves_the_selection_as_it_was() {
    let (browser, _) = loaded(entries()).step(Input::Click(name("b.txt"), Modifier::None), at(0));
    let (browser, _) = browser.step(reveal("/home/p", &["gone.txt"]), at(0));
    assert_eq!(selected(&browser), vec!["b.txt"]);
}

#[test]
fn names_that_arrive_in_a_later_listing_or_change_are_selected_then() {
    let (browser, _) =
        loaded(vec![file("a.txt")]).step(reveal("/home/p", &["a.txt", "new.txt"]), at(0));
    assert_eq!(selected(&browser), vec!["a.txt"]);

    // The folder is read again and now holds the second name too.
    let (browser, _) = browser.step(
        Input::Listed(home(), Ok(vec![file("a.txt"), file("new.txt")])),
        at(0),
    );
    assert_eq!(selected(&browser), vec!["a.txt", "new.txt"]);

    // Or the watcher reports it.
    let (browser, _) = loaded(vec![file("a.txt")]).step(reveal("/home/p", &["new.txt"]), at(0));
    assert_eq!(selected(&browser), Vec::<String>::new());
    let (browser, _) = browser.step(
        Input::Changed(home(), Change::Added(file("new.txt"))),
        at(0),
    );
    assert_eq!(selected(&browser), vec!["new.txt"]);
}

#[test]
fn a_reveal_while_the_listing_is_pending_selects_when_it_arrives() {
    let (browser, _) = Browser::new(home());
    let (browser, _) = browser.step(reveal("/home/p", &["b.txt"]), at(0));
    assert_eq!(selected(&browser), Vec::<String>::new());
    let (browser, _) = browser.step(Input::Listed(home(), Ok(entries())), at(0));
    assert_eq!(selected(&browser), vec!["b.txt"]);
    assert_eq!(browser.cursor(), Some(&name("b.txt")));
}

#[test]
fn a_reveal_in_another_folder_is_a_visit_that_selects_once_that_folder_is_listed() {
    let there = dir("/srv/data");
    let (browser, effects) = loaded(entries()).step(reveal("/srv/data", &["x.txt"]), at(0));
    assert_eq!(path_of(browser.location()), "/srv/data");
    assert_eq!(browser.can_go_back(), Availability::Available);
    assert_eq!(effects.first(), Some(&Effect::Unwatch(home())));
    assert!(effects.contains(&Effect::List(there.clone())));

    // The old folder's answer is for a place we left.
    let (browser, _) = browser.step(Input::Listed(home(), Ok(entries())), at(0));
    assert_eq!(selected(&browser), Vec::<String>::new());

    let (browser, _) = browser.step(
        Input::Listed(there, Ok(vec![file("w.txt"), file("x.txt")])),
        at(0),
    );
    assert_eq!(selected(&browser), vec!["x.txt"]);

    // Back goes where the window was.
    let (browser, _) = browser.step(Input::Back, at(0));
    assert_eq!(path_of(browser.location()), "/home/p");
}

#[test]
fn selecting_something_ends_the_wait_for_names_that_have_not_arrived() {
    let (browser, _) = loaded(vec![file("a.txt")]).step(reveal("/home/p", &["new.txt"]), at(0));
    let (browser, _) = browser.step(Input::Click(name("a.txt"), Modifier::None), at(0));
    let (browser, _) = browser.step(
        Input::Changed(home(), Change::Added(file("new.txt"))),
        at(0),
    );
    assert_eq!(selected(&browser), vec!["a.txt"]);
}

#[test]
fn leaving_the_folder_drops_the_wait() {
    let (browser, _) = Browser::new(home());
    let (browser, _) = browser.step(reveal("/home/p", &["a.txt"]), at(0));
    let (browser, _) = browser.step(Input::Go(dir("/tmp")), at(0));
    let (browser, _) = browser.step(Input::Go(home()), at(0));
    let (browser, _) = browser.step(Input::Listed(home(), Ok(entries())), at(0));
    assert_eq!(selected(&browser), Vec::<String>::new());
}
