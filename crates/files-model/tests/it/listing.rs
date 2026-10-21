//! Answers arriving: stale ones, failures and live changes.

use files_model::{Change, Effect, Input, ListFailure, Listing, Modifier};

use crate::common::*;

#[test]
fn an_answer_for_a_place_already_left_is_ignored() {
    let (browser, _) = run(loaded(vec![]), vec![Input::Go(dir("/x"))]);
    let (after, effects) = browser
        .clone()
        .step(Input::Listed(home(), Ok(vec![file("old")])), at(0));
    assert_eq!(after, browser);
    assert!(effects.is_empty());
    assert_eq!(after.listing(), &Listing::Loading);
}

#[test]
fn a_failed_listing_is_kept_with_its_reason_and_clears_the_rows() {
    let browser = loaded(vec![file("a")]);
    let (browser, _) = browser.step(Input::Listed(home(), Err(ListFailure::NoPermission)), at(0));
    assert_eq!(
        browser.listing(),
        &Listing::Failed(ListFailure::NoPermission)
    );
    assert!(browser.rows().is_empty());
    assert_eq!(
        ListFailure::NoPermission.to_string(),
        "You don't have permission to open that folder."
    );
}

#[test]
fn reload_asks_to_list_again_and_keeps_the_rows_meanwhile() {
    let (browser, effects) = loaded(vec![file("a")]).step(Input::Reload, at(0));
    assert_eq!(effects, vec![Effect::List(home())]);
    assert_eq!(shown(&browser), vec!["a"]);
}

#[test]
fn a_reload_keeps_the_selection_of_names_still_there() {
    let browser = loaded(vec![file("a"), file("b"), file("c")]);
    let (browser, _) = run(
        browser,
        vec![
            Input::Click(name("a"), Modifier::None),
            Input::Click(name("b"), Modifier::Toggle),
            Input::Listed(home(), Ok(vec![file("b"), file("c"), file("d")])),
        ],
    );
    assert_eq!(selected(&browser), vec!["b"]);
    assert_eq!(browser.cursor(), Some(&name("b")));
    assert_eq!(browser.selection().anchor(), Some(&name("b")));
}

#[test]
fn live_changes_add_update_and_remove_rows_and_keep_the_selection() {
    let browser = loaded(vec![file("b"), file("c")]);
    let (browser, _) = browser.step(Input::Click(name("c"), Modifier::None), at(0));
    let steps: Vec<(Change, Vec<&str>, Vec<&str>)> = vec![
        (Change::Added(file("a")), vec!["a", "b", "c"], vec!["c"]),
        (Change::Added(file("a")), vec!["a", "b", "c"], vec!["c"]),
        (
            Change::Updated(file("b").modified_at(std::time::UNIX_EPOCH)),
            vec!["a", "b", "c"],
            vec!["c"],
        ),
        (Change::Removed(name("a")), vec!["b", "c"], vec!["c"]),
        (Change::Removed(name("c")), vec!["b"], vec![]),
    ];
    let mut browser = browser;
    for (change, rows, chosen) in steps {
        browser = browser
            .step(Input::Changed(home(), change.clone()), at(0))
            .0;
        assert_eq!(shown(&browser), rows, "{change:?}");
        assert_eq!(selected(&browser), chosen, "{change:?}");
    }
}

#[test]
fn an_update_replaces_the_entry_it_names() {
    let stamp = std::time::UNIX_EPOCH + std::time::Duration::from_secs(5);
    let browser = loaded(vec![file("a")]);
    let (browser, _) = browser.step(
        Input::Changed(home(), Change::Updated(file("a").modified_at(stamp))),
        at(0),
    );
    assert_eq!(browser.rows().len(), 1);
    assert_eq!(browser.rows()[0].modified, Some(stamp));
}

#[test]
fn rescan_asks_to_list_again() {
    let (_, effects) = loaded(vec![]).step(Input::Changed(home(), Change::Rescan), at(0));
    assert_eq!(effects, vec![Effect::List(home())]);
}

#[test]
fn changes_for_another_place_or_before_the_first_listing_are_ignored() {
    let (loading, _) = files_model::Browser::new(home());
    let (after, _) = loading
        .clone()
        .step(Input::Changed(home(), Change::Added(file("a"))), at(0));
    assert_eq!(after, loading);
    let browser = loaded(vec![]);
    let (after, _) = browser
        .clone()
        .step(Input::Changed(dir("/x"), Change::Added(file("a"))), at(0));
    assert_eq!(after, browser);
}
