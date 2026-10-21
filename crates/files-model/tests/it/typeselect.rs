//! Typing a name to jump to it.

use files_model::{Input, Modifier};

use crate::common::*;

fn names() -> files_model::Browser {
    loaded(["apple", "Avocado", "banana", "blueberry"].map(file).to_vec())
}

fn types(browser: files_model::Browser, keys: &[(char, u64)]) -> files_model::Browser {
    keys.iter()
        .fold(browser, |b, (c, ms)| b.step(Input::Type(*c), at(*ms)).0)
}

#[test]
fn keys_within_a_second_extend_the_word_and_a_pause_starts_a_new_one() {
    // (keys with the time of each in ms, the name selected after)
    let table: Vec<(Vec<(char, u64)>, &str)> = vec![
        (vec![('a', 0)], "apple"),
        (vec![('a', 0), ('v', 500)], "Avocado"),
        (vec![('b', 0), ('l', 999)], "blueberry"),
        (vec![('b', 0), ('l', 1000)], "blueberry"),
        (vec![('b', 0), ('a', 1001)], "apple"),
        (vec![('b', 0), ('a', 400), ('n', 800)], "banana"),
        (vec![('b', 0), ('l', 600), ('u', 1500), ('a', 3100)], "apple"),
        (vec![('B', 0), ('L', 100)], "blueberry"),
    ];
    for (keys, want) in table {
        let browser = types(names(), &keys);
        assert_eq!(selected(&browser), vec![want], "{keys:?}");
    }
}

#[test]
fn a_word_with_no_match_keeps_the_selection() {
    let browser = types(names(), &[('b', 0)]);
    assert_eq!(selected(&browser), vec!["banana"]);
    let browser = types(browser, &[('q', 100)]);
    assert_eq!(selected(&browser), vec!["banana"]);
    assert_eq!(browser.typed(), "bq");
}

#[test]
fn typing_selects_only_the_match_and_moves_the_cursor() {
    let browser = types(names(), &[('a', 0)]);
    assert_eq!(browser.cursor(), Some(&name("apple")));
    assert_eq!(browser.rows().len(), 4);
}

#[test]
fn a_click_ends_the_word() {
    let browser = types(names(), &[('b', 0)]);
    let (browser, _) = browser.step(Input::Click(name("apple"), Modifier::None), at(100));
    let browser = types(browser, &[('b', 200)]);
    assert_eq!(browser.typed(), "b");
}

#[test]
fn control_characters_are_not_letters() {
    let browser = types(names(), &[('\n', 0), ('\u{1b}', 10)]);
    assert!(browser.selection().is_empty());
    assert_eq!(browser.typed(), "");
}
