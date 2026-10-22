//! Tabs: opening, closing and switching them, and an answer finding the tab that asked.

use files_core::Location;
use files_model::{Effect, Input, TabId, Tabbed, Tabs};

use crate::common::{at, dir, file, folder, home, name};

/// Three tabs on `/home/p`, `/a` and `/b`, left to right, with the third in front.
fn three() -> (Tabs, [TabId; 3]) {
    let (tabs, _) = Tabs::new(home());
    let first = tabs.front_id();
    let (tabs, _) = tabs.step(Input::Go(dir("/a")), at(0));
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let second = tabs.front_id();
    let (tabs, _) = tabs.step(Input::Go(dir("/b")), at(0));
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let third = tabs.front_id();
    (tabs, [first, second, third])
}

fn ids(tabs: &Tabs) -> Vec<TabId> {
    tabs.iter().map(|tab| tab.id()).collect()
}

fn places(tabs: &Tabs) -> Vec<Location> {
    tabs.iter()
        .map(|tab| tab.browser().location().clone())
        .collect()
}

#[test]
fn a_new_tab_opens_on_the_same_place_next_to_the_front_one_and_in_front() {
    let (tabs, effects) = Tabs::new(home());
    assert_eq!(tabs.len(), 1);
    assert!(effects.iter().all(|e| e.tab == TabId::FIRST));
    let (tabs, effects) = tabs.step(Input::NewTab, at(0));
    let new = tabs.front_id();
    assert_ne!(new, TabId::FIRST);
    assert_eq!(ids(&tabs), vec![TabId::FIRST, new]);
    assert_eq!(places(&tabs), vec![home(), home()]);
    // Its effects load it, and every one is tagged with it.
    let want: Vec<Tabbed> = Tabbed::all(
        new,
        vec![
            Effect::List(home()),
            Effect::Watch(home()),
            Effect::LoadPrefs(home()),
        ],
    );
    assert_eq!(effects, want);
}

#[test]
fn a_new_tab_goes_next_to_the_front_tab_not_at_the_end() {
    let (tabs, [first, second, third]) = three();
    let (tabs, _) = tabs.step(Input::ShowTab(first), at(0));
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let new = tabs.front_id();
    assert_eq!(ids(&tabs), vec![first, new, second, third]);
}

#[test]
fn each_tab_keeps_its_own_place() {
    let (tabs, [first, second, third]) = three();
    assert_eq!(places(&tabs), vec![dir("/a"), dir("/b"), dir("/b")]);
    let (tabs, _) = tabs.step(Input::ShowTab(first), at(0));
    assert_eq!(tabs.front().location(), &dir("/a"));
    let (tabs, _) = tabs.step(Input::Go(dir("/c")), at(0));
    assert_eq!(places(&tabs), vec![dir("/c"), dir("/b"), dir("/b")]);
    assert_eq!(ids(&tabs), vec![first, second, third]);
}

#[test]
fn a_listing_for_a_background_tab_lands_in_that_tab() {
    let (tabs, [first, _, third]) = three();
    assert_eq!(tabs.front_id(), third);
    let listed = Input::Listed(dir("/a"), Ok(vec![file("a.txt")]));
    let (tabs, effects) = tabs.heard(first, listed, at(0));
    assert!(effects.is_empty());
    let rows = |id: TabId| tabs.get(id).unwrap().browser().rows().len();
    assert_eq!(rows(first), 1);
    assert_eq!(rows(third), 0, "the front tab is untouched");
    assert_eq!(tabs.front().rows().len(), 0);
}

#[test]
fn a_listing_for_the_place_a_tab_has_left_is_ignored_by_that_tab() {
    let (tabs, [first, _, _]) = three();
    // The first tab is at /a, not /zzz.
    let stale = Input::Listed(dir("/zzz"), Ok(vec![file("x")]));
    let (tabs, _) = tabs.heard(first, stale, at(0));
    assert_eq!(tabs.get(first).unwrap().browser().rows().len(), 0);
}

#[test]
fn two_tabs_on_one_folder_hear_only_their_own_answers() {
    let (tabs, _) = Tabs::new(home());
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let front = tabs.front_id();
    let listed = Input::Listed(home(), Ok(vec![file("a"), file("b")]));
    let (tabs, _) = tabs.heard(TabId::FIRST, listed, at(0));
    assert_eq!(tabs.get(TabId::FIRST).unwrap().browser().rows().len(), 2);
    assert_eq!(tabs.get(front).unwrap().browser().rows().len(), 0);
}

#[test]
fn an_answer_for_a_closed_tab_is_dropped() {
    let (tabs, [first, _, _]) = three();
    let (tabs, _) = tabs.step(Input::ShowTab(first), at(0));
    let (tabs, _) = tabs.step(Input::CloseTab, at(0));
    assert!(tabs.get(first).is_none());
    let before = tabs.clone();
    let (after, effects) = tabs.heard(first, Input::Reload, at(0));
    assert_eq!(after, before);
    assert!(effects.is_empty());
}

#[test]
fn closing_the_front_tab_brings_forward_the_one_on_its_right_else_its_left() {
    let (tabs, [first, second, third]) = three();
    let (tabs, _) = tabs.step(Input::ShowTab(second), at(0));
    let (tabs, effects) = tabs.step(Input::CloseTab, at(0));
    assert_eq!(ids(&tabs), vec![first, third]);
    assert_eq!(tabs.front_id(), third);
    // The closed tab stops watching its place.
    assert_eq!(
        effects,
        vec![Tabbed::new(second, Effect::Unwatch(dir("/b")))]
    );
    // The last tab of the row closing: its left neighbour comes forward.
    let (tabs, _) = tabs.step(Input::CloseTab, at(0));
    assert_eq!(ids(&tabs), vec![first]);
    assert_eq!(tabs.front_id(), first);
}

#[test]
fn closing_the_last_tab_asks_for_the_window_to_close() {
    let (tabs, _) = Tabs::new(home());
    let (after, effects) = tabs.clone().step(Input::CloseTab, at(0));
    assert_eq!(after, tabs);
    assert_eq!(
        effects,
        vec![Tabbed::new(TabId::FIRST, Effect::CloseWindow)]
    );
}

#[test]
fn next_and_previous_wrap_round() {
    let (tabs, [first, second, third]) = three();
    let cases = [
        (Input::NextTab, third, first),
        (Input::PreviousTab, third, second),
    ];
    for (input, from, to) in cases {
        let (tabs, _) = tabs.clone().step(Input::ShowTab(from), at(0));
        let (tabs, _) = tabs.step(input.clone(), at(0));
        assert_eq!(tabs.front_id(), to, "{input:?} from {from:?}");
        assert_eq!(ids(&tabs), vec![first, second, third], "the order stays");
    }
    let (tabs, _) = tabs.step(Input::ShowTab(first), at(0));
    let (tabs, _) = tabs.step(Input::PreviousTab, at(0));
    assert_eq!(tabs.front_id(), third);
}

#[test]
fn showing_a_tab_keeps_the_order_whichever_way_it_jumps() {
    let (tabs, [first, second, third]) = three();
    for target in [first, second, third] {
        for start in [first, second, third] {
            let (shown, _) = tabs.clone().step(Input::ShowTab(start), at(0));
            let (shown, _) = shown.step(Input::ShowTab(target), at(0));
            assert_eq!(shown.front_id(), target);
            assert_eq!(ids(&shown), vec![first, second, third]);
        }
    }
    let (same, effects) = tabs.clone().step(Input::ShowTab(TabId::FIRST), at(0));
    assert_eq!(ids(&same), vec![first, second, third]);
    assert!(effects.is_empty());
}

#[test]
fn a_single_tab_does_not_switch() {
    let (tabs, _) = Tabs::new(home());
    for input in [
        Input::NextTab,
        Input::PreviousTab,
        Input::ShowTab(TabId::FIRST),
    ] {
        let (after, effects) = tabs.clone().step(input, at(0));
        assert_eq!(after, tabs);
        assert!(effects.is_empty());
    }
}

#[test]
fn open_in_new_tab_opens_the_selected_folder_and_only_a_folder() {
    let (tabs, _) = Tabs::new(home());
    let listed = Input::Listed(home(), Ok(vec![folder("Projects"), file("a.txt")]));
    let (tabs, _) = tabs.heard(TabId::FIRST, listed, at(0));
    // A file: nothing.
    let (tabs, _) = tabs.step(
        Input::Click(name("a.txt"), files_model::Modifier::None),
        at(0),
    );
    let (tabs, effects) = tabs.step(Input::OpenInNewTab, at(0));
    assert_eq!(tabs.len(), 1);
    assert!(effects.is_empty());
    // A folder: a tab on it, in front, and the first tab stays where it is.
    let (tabs, _) = tabs.step(
        Input::Click(name("Projects"), files_model::Modifier::None),
        at(0),
    );
    let (tabs, effects) = tabs.step(Input::OpenInNewTab, at(0));
    let new = tabs.front_id();
    assert_eq!(tabs.len(), 2);
    assert_eq!(tabs.front().location(), &dir("/home/p/Projects"));
    assert_eq!(
        tabs.get(TabId::FIRST).unwrap().browser().location(),
        &home()
    );
    assert_eq!(
        effects[0],
        Tabbed::new(new, Effect::List(dir("/home/p/Projects")))
    );
}

#[test]
fn the_clipboard_and_undo_belong_to_the_window_not_one_tab() {
    let (tabs, _) = Tabs::new(home());
    let listed = Input::Listed(home(), Ok(vec![file("a.txt")]));
    let (tabs, _) = tabs.heard(TabId::FIRST, listed, at(0));
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let (tabs, _) = tabs.step(Input::ShowTab(TabId::FIRST), at(0));
    let (tabs, _) = tabs.step(
        Input::Click(name("a.txt"), files_model::Modifier::None),
        at(0),
    );
    let (tabs, _) = tabs.step(Input::Copy, at(0));
    let (tabs, _) = tabs.step(Input::NextTab, at(0));
    assert_eq!(tabs.front().clipboard().len(), 1);
}

#[test]
fn the_disks_changing_is_heard_by_every_tab() {
    let (tabs, _) = three();
    let (tabs, effects) = tabs.broadcast(&Input::Volumes(Vec::new()), at(0));
    assert!(effects.is_empty());
    assert_eq!(tabs.len(), 3);
}

#[test]
fn a_tab_with_quick_look_up_asks_for_its_preview_when_it_comes_forward() {
    let (tabs, _) = Tabs::new(home());
    let listed = Input::Listed(home(), Ok(vec![file("a.txt")]));
    let (tabs, _) = tabs.heard(TabId::FIRST, listed, at(0));
    let (tabs, _) = tabs.step(
        Input::Click(name("a.txt"), files_model::Modifier::None),
        at(0),
    );
    let (tabs, _) = tabs.step(Input::QuickLook, at(0));
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let (_, effects) = tabs.step(Input::ShowTab(TabId::FIRST), at(0));
    assert_eq!(effects.len(), 1);
    assert!(matches!(effects[0].effect, Effect::Look(_)));
    assert_eq!(effects[0].tab, TabId::FIRST);
}
