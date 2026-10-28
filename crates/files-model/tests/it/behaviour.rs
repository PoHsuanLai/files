//! The person's choices about how the window behaves: folders on top, the two warnings, and a
//! choice changed while the window is open.

use files_core::{Behaviour, Entry, FolderPrefs, OpPlan, Sort, SortKey, SortOrder};
use files_model::{Browser, Effect, Input, Modifier, Question, Tabs, arrange};

use crate::changes::{home_folder, trash_browser};
use crate::common::*;

fn behaving(behaviour: Behaviour, entries: Vec<Entry>) -> Browser {
    loaded(entries).with_behaviour(behaviour)
}

fn first() -> Behaviour {
    Behaviour {
        folders_first: true,
        ..Behaviour::default()
    }
}

fn ordered(entries: &[Entry], sort: Sort, folders_first: bool) -> Vec<String> {
    let mut prefs = FolderPrefs::default();
    prefs.sort = sort;
    let behaviour = Behaviour {
        folders_first,
        ..Behaviour::default()
    };
    arrange(entries, &prefs, true, behaviour)
        .into_iter()
        .map(|at| entries[at].name.display().into_owned())
        .collect()
}

#[test]
fn the_defaults_are_finders() {
    assert_eq!(
        Behaviour::default(),
        Behaviour {
            folders_first: false,
            ask_before_emptying_trash: true,
            ask_before_changing_extension: true,
        }
    );
}

#[test]
fn folders_come_first_only_when_asked_and_only_by_name() {
    let entries = [file("b"), folder("c"), file("a"), folder("d")];
    let by = |key, order| Sort { key, order };
    let name_up = by(SortKey::Name, SortOrder::Ascending);
    let name_down = by(SortKey::Name, SortOrder::Descending);
    let size_up = by(SortKey::Size, SortOrder::Ascending);
    let table = [
        ("mixed", name_up, false, ["a", "b", "c", "d"]),
        ("first", name_up, true, ["c", "d", "a", "b"]),
        ("descending", name_down, true, ["d", "c", "b", "a"]),
        ("by size", size_up, true, ["c", "d", "a", "b"]),
    ];
    for (label, sort, folders_first, want) in table {
        assert_eq!(ordered(&entries, sort, folders_first), want, "{label}");
    }
}

#[test]
fn a_changed_choice_arranges_the_rows_again_and_a_new_tab_keeps_it() {
    let browser = loaded(vec![file("a"), folder("b")]);
    assert_eq!(shown(&browser), vec!["a", "b"]);
    let (browser, effects) = browser.step(Input::Behave(first()), at(0));
    assert!(effects.is_empty());
    assert_eq!(shown(&browser), vec!["b", "a"]);
    assert_eq!(browser.behaviour(), first());

    let (tabs, _) = Tabs::new(home());
    let tabs = tabs.with_behaviour(first());
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    assert_eq!(tabs.front().behaviour(), first());
    let (tabs, _) = tabs.broadcast(&Input::Behave(Behaviour::default()), at(0));
    let all_default = tabs
        .iter()
        .all(|tab| tab.browser().behaviour() == Behaviour::default());
    assert!(all_default);
}

#[test]
fn emptying_the_trash_asks_unless_the_person_chose_not_to_be_asked() {
    let table = [
        ("asked", true, Question::EmptyTrash, vec![]),
        (
            "not asked",
            false,
            Question::None,
            vec![Effect::Run(OpPlan::EmptyTrash)],
        ),
    ];
    for (label, asks, question, want) in table {
        let behaviour = Behaviour {
            ask_before_emptying_trash: asks,
            ..Behaviour::default()
        };
        let (browser, effects) = trash_browser()
            .with_behaviour(behaviour)
            .step(Input::EmptyTrash, at(0));
        assert_eq!(browser.question(), &question, "{label}");
        assert_eq!(effects, want, "{label}");
    }
}

#[test]
fn another_program_asking_to_empty_the_trash_is_always_asked_about() {
    let behaviour = Behaviour {
        ask_before_emptying_trash: false,
        ..Behaviour::default()
    };
    let (browser, effects) = loaded(Vec::new())
        .with_behaviour(behaviour)
        .step(Input::AskEmptyTrash, at(0));
    assert_eq!(browser.question(), &Question::EmptyTrash);
    assert!(effects.is_empty());
}

fn renamed(
    behaviour: Behaviour,
    entries: Vec<Entry>,
    from: &str,
    draft: &str,
) -> (Browser, Vec<Effect>) {
    let browser = behaving(behaviour, entries);
    let (browser, _) = run(
        browser,
        vec![
            Input::Click(name(from), Modifier::None),
            Input::BeginRename,
            Input::EditDraft(draft.into()),
        ],
    );
    browser.step(Input::CommitRename, at(0))
}

fn rename_to(from: &str, to: &str) -> Effect {
    Effect::Rename {
        folder: home_folder(),
        from: name(from),
        to: name(to),
    }
}

#[test]
fn changing_an_extension_asks_and_the_answer_decides() {
    let entries = || vec![file("a.txt"), file("b.txt"), file("c"), folder("d.x")];
    // (label, from, draft, the name "Keep" gives)
    let table = [
        ("another extension", "a.txt", "a.md", "a.txt"),
        ("name and extension", "a.txt", "z.md", "z.txt"),
        ("extension dropped", "a.txt", "z", "z.txt"),
        ("extension added", "c", "c.md", "c"),
        ("the kept name is taken", "a.txt", "b.md", "a.txt"),
    ];
    for (label, from, draft, kept) in table {
        let (asked, effects) = renamed(Behaviour::default(), entries(), from, draft);
        assert!(effects.is_empty(), "{label}");
        assert_eq!(asked.rename(), &files_model::Rename::Idle, "{label}");
        let Question::ChangeExtension(change) = asked.question().clone() else {
            panic!("{label}: no question: {:?}", asked.question());
        };
        assert_eq!(change.kept, name(kept), "{label}");

        let (used, effects) = asked.clone().step(Input::Confirm, at(0));
        assert_eq!(effects, vec![rename_to(from, draft)], "{label}: use");
        assert_eq!(used.question(), &Question::None, "{label}");

        let (_, effects) = asked.step(Input::Dismiss, at(0));
        let want = if kept == from {
            vec![]
        } else {
            vec![rename_to(from, kept)]
        };
        assert_eq!(effects, want, "{label}: keep");
    }
}

#[test]
fn a_rename_that_leaves_the_extension_or_is_not_asked_about_goes_straight_on() {
    let entries = || vec![file("a.txt"), folder("d.x")];
    let skip = Behaviour {
        ask_before_changing_extension: false,
        ..Behaviour::default()
    };
    let table = [
        ("same extension", Behaviour::default(), "a.txt", "b.txt"),
        ("only the case", Behaviour::default(), "a.txt", "a.TXT"),
        ("a folder", Behaviour::default(), "d.x", "d.y"),
        ("not asked", skip, "a.txt", "a.md"),
    ];
    for (label, behaviour, from, draft) in table {
        let (browser, effects) = renamed(behaviour, entries(), from, draft);
        assert_eq!(browser.question(), &Question::None, "{label}");
        assert_eq!(effects, vec![rename_to(from, draft)], "{label}");
    }
}
