//! Click, extend, toggle, select all and the arrow keys.

use files_model::{Direction, Input, Modifier};

use crate::common::*;

fn five() -> files_model::Browser {
    loaded(["a", "b", "c", "d", "e"].map(file).to_vec())
}

fn click(text: &str, modifier: Modifier) -> Input {
    Input::Click(name(text), modifier)
}

fn key(direction: Direction, modifier: Modifier) -> Input {
    Input::MoveCursor(direction, modifier)
}

#[test]
fn selection_follows_clicks_and_keys() {
    use Direction::{Down, End, Home, Up};
    use Modifier::{Extend, None as Plain, Toggle};
    // (inputs, selected, cursor)
    let table: Vec<(Vec<Input>, Vec<&str>, Option<&str>)> = vec![
        (vec![click("b", Plain)], vec!["b"], Some("b")),
        (
            vec![click("b", Plain), click("d", Extend)],
            vec!["b", "c", "d"],
            Some("d"),
        ),
        (
            vec![click("b", Plain), click("d", Extend), click("a", Extend)],
            vec!["a", "b"],
            Some("a"),
        ),
        (vec![click("c", Extend)], vec!["c"], Some("c")),
        (
            vec![click("a", Plain), click("c", Toggle)],
            vec!["a", "c"],
            Some("c"),
        ),
        (
            vec![click("a", Plain), click("c", Toggle), click("c", Toggle)],
            vec!["a"],
            Some("c"),
        ),
        (
            vec![click("a", Plain), click("b", Toggle), click("d", Plain)],
            vec!["d"],
            Some("d"),
        ),
        (vec![click("zzz", Plain)], vec![], None),
        (vec![Input::SelectAll], vec!["a", "b", "c", "d", "e"], None),
        (vec![click("c", Plain), Input::SelectNone], vec![], None),
        (vec![key(Down, Plain)], vec!["a"], Some("a")),
        (vec![key(Up, Plain)], vec!["e"], Some("e")),
        (
            vec![key(Down, Plain), key(Down, Plain), key(Down, Plain)],
            vec!["c"],
            Some("c"),
        ),
        (
            vec![click("a", Plain), key(Up, Plain)],
            vec!["a"],
            Some("a"),
        ),
        (
            vec![click("d", Plain), key(End, Plain), key(Down, Plain)],
            vec!["e"],
            Some("e"),
        ),
        (
            vec![click("d", Plain), key(Home, Plain)],
            vec!["a"],
            Some("a"),
        ),
        (
            vec![click("b", Plain), key(Down, Extend), key(Down, Extend)],
            vec!["b", "c", "d"],
            Some("d"),
        ),
        (
            vec![click("b", Plain), key(Down, Toggle)],
            vec!["b"],
            Some("c"),
        ),
        (
            vec![click("b", Plain), key(Down, Toggle), click("d", Extend)],
            vec!["b", "c", "d"],
            Some("d"),
        ),
    ];
    for (inputs, chosen, cursor) in table {
        let (browser, _) = run(five(), inputs.clone());
        assert_eq!(selected(&browser), chosen, "{inputs:?}");
        assert_eq!(
            browser.cursor().map(|n| n.display().into_owned()),
            cursor.map(String::from),
            "{inputs:?}"
        );
    }
}

#[test]
fn selecting_all_keeps_the_anchor_for_a_later_extend() {
    let (browser, _) = run(
        five(),
        vec![
            click("b", Modifier::None),
            Input::SelectAll,
            click("d", Modifier::Extend),
        ],
    );
    assert_eq!(selected(&browser), vec!["b", "c", "d"]);
}

#[test]
fn selection_has_no_effects() {
    let (_, effects) = run(five(), vec![click("a", Modifier::None), Input::SelectAll]);
    assert!(effects.is_empty());
}

#[test]
fn only_one_selected_is_single() {
    let (one, _) = run(five(), vec![click("a", Modifier::None)]);
    assert_eq!(one.selection().single(), Some(&name("a")));
    let (two, _) = run(
        five(),
        vec![click("a", Modifier::None), click("b", Modifier::Toggle)],
    );
    assert_eq!(two.selection().single(), None);
}

#[test]
fn the_keyboard_does_nothing_in_an_empty_folder() {
    let (browser, _) = run(
        loaded(vec![]),
        vec![key(Direction::Down, Modifier::None), Input::SelectAll],
    );
    assert!(browser.selection().is_empty());
}
