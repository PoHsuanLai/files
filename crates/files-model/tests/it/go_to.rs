//! Go to Folder: the sheet, the typed path, the check and the arrival.

use files_core::{FolderPath, Location};
use files_model::{Browser, Effect, GoTo, GoToProblem, Input, ListFailure, expand};

use crate::common::*;

fn path(text: &str) -> FolderPath {
    FolderPath::new(text).unwrap()
}

fn home_path() -> FolderPath {
    path("/home/p")
}

/// A browser at `/home/p` that knows home is `/home/p`.
fn knowing_home() -> Browser {
    loaded(vec![folder("Projects")]).with_home(home_path())
}

fn asking(draft: &str, problem: Option<GoToProblem>, checking: Option<&str>) -> GoTo {
    GoTo::Asking {
        draft: draft.to_owned(),
        problem,
        checking: checking.map(path),
    }
}

fn typed(text: &str) -> Input {
    Input::GoToTyped(text.to_owned())
}

/// A label, the typed text, the home folder, and the folder (or problem) it means.
type GoToCase<'a> = (
    &'a str,
    &'a str,
    Option<&'a FolderPath>,
    Result<&'a str, GoToProblem>,
);

#[test]
fn a_typed_path_means_a_folder() {
    let home = home_path();
    let cases: Vec<GoToCase> = vec![
        ("a full path", "/usr/share", Some(&home), Ok("/usr/share")),
        (
            "spaces around it",
            "  /usr/share \n",
            None,
            Ok("/usr/share"),
        ),
        ("a trailing slash", "/usr/share/", None, Ok("/usr/share")),
        ("tilde alone", "~", Some(&home), Ok("/home/p")),
        ("tilde and a slash", "~/", Some(&home), Ok("/home/p")),
        (
            "below home",
            "~/Projects/x",
            Some(&home),
            Ok("/home/p/Projects/x"),
        ),
        (
            "tilde with no home",
            "~/x",
            None,
            Err(GoToProblem::NotFound),
        ),
        (
            "another person's tilde",
            "~root",
            Some(&home),
            Err(GoToProblem::NotFound),
        ),
        (
            "a relative path",
            "Projects",
            Some(&home),
            Err(GoToProblem::NotFound),
        ),
        ("nothing", "", Some(&home), Err(GoToProblem::NotFound)),
    ];
    for (label, text, home, want) in cases {
        let want = want.map(path);
        assert_eq!(expand(text, home), want, "{label}");
    }
}

#[test]
fn the_sheet_opens_on_the_folder_shown() {
    let (browser, effects) = knowing_home().step(Input::GoToFolder, at(0));
    assert_eq!(browser.go_to(), &asking("/home/p", None, None));
    assert!(effects.is_empty());

    // In the Trash there is no folder: home is offered.
    let (trash, _) = Browser::new(Location::Trash);
    let (trash, _) = trash.with_home(home_path()).step(Input::GoToFolder, at(0));
    assert_eq!(trash.go_to(), &asking("/home/p", None, None));
}

#[test]
fn typing_changes_the_text_and_forgets_the_complaint() {
    let (browser, _) = run(
        knowing_home(),
        vec![Input::GoToFolder, typed("nope"), Input::GoToSubmit],
    );
    assert_eq!(
        browser.go_to(),
        &asking("nope", Some(GoToProblem::NotFound), None)
    );
    let (browser, _) = browser.step(typed("/tmp"), at(0));
    assert_eq!(browser.go_to(), &asking("/tmp", None, None));
}

#[test]
fn return_asks_whether_the_folder_is_there_and_waits() {
    let (browser, effects) = run(
        knowing_home(),
        vec![Input::GoToFolder, typed("~/Projects"), Input::GoToSubmit],
    );
    assert_eq!(effects, vec![Effect::CheckFolder(path("/home/p/Projects"))]);
    assert_eq!(
        browser.go_to(),
        &asking("~/Projects", None, Some("/home/p/Projects"))
    );
    assert_eq!(path_of(browser.location()), "/home/p", "not there yet");
}

#[test]
fn a_path_that_is_no_path_stays_open_with_the_words() {
    let (browser, effects) = run(
        knowing_home(),
        vec![Input::GoToFolder, typed("Projects"), Input::GoToSubmit],
    );
    assert!(effects.is_empty());
    assert_eq!(
        browser.go_to(),
        &asking("Projects", Some(GoToProblem::NotFound), None)
    );
    assert_eq!(
        GoToProblem::NotFound.to_string(),
        "The folder can't be found."
    );
}

#[test]
fn the_answer_decides_between_arriving_and_staying() {
    let there = path("/home/p/Projects");
    let ask = vec![Input::GoToFolder, typed("~/Projects"), Input::GoToSubmit];
    // label, answer, whether we arrive, the sheet after
    let cases: Vec<(&str, Result<(), ListFailure>, Option<GoTo>)> = vec![
        ("it is there", Ok(()), None),
        (
            "not found",
            Err(ListFailure::NotFound),
            Some(asking("~/Projects", Some(GoToProblem::NotFound), None)),
        ),
        (
            "a file",
            Err(ListFailure::NotAFolder),
            Some(asking("~/Projects", Some(GoToProblem::NotFound), None)),
        ),
        (
            "not allowed",
            Err(ListFailure::NoPermission),
            Some(asking("~/Projects", Some(GoToProblem::NoPermission), None)),
        ),
    ];
    for (label, answer, sheet) in cases {
        let (browser, _) = run(knowing_home(), ask.clone());
        let (after, effects) = browser.step(Input::FolderChecked(there.clone(), answer), at(0));
        match sheet {
            None => {
                assert_eq!(after.go_to(), &GoTo::Closed, "{label}");
                assert_eq!(path_of(after.location()), "/home/p/Projects", "{label}");
                assert!(effects.contains(&Effect::List(after.location().clone())));
            }
            Some(sheet) => {
                assert_eq!(after.go_to(), &sheet, "{label}");
                assert_eq!(path_of(after.location()), "/home/p", "{label}");
                assert!(effects.is_empty(), "{label}");
            }
        }
    }
}

#[test]
fn an_answer_nobody_waits_for_is_ignored() {
    let ask = vec![Input::GoToFolder, typed("/tmp"), Input::GoToSubmit];
    let (browser, _) = run(knowing_home(), ask);
    // Another path than the one asked about.
    let (same, effects) = browser
        .clone()
        .step(Input::FolderChecked(path("/var"), Ok(())), at(0));
    assert_eq!(same, browser);
    assert!(effects.is_empty());
    // The sheet was put away meanwhile.
    let (gone, _) = browser.step(Input::GoToCancel, at(0));
    let (still, effects) = gone
        .clone()
        .step(Input::FolderChecked(path("/tmp"), Ok(())), at(0));
    assert_eq!(still, gone);
    assert!(effects.is_empty());
}

#[test]
fn escape_puts_the_sheet_away_and_stays_put() {
    let (browser, _) = run(knowing_home(), vec![Input::GoToFolder, Input::GoToCancel]);
    assert_eq!(browser.go_to(), &GoTo::Closed);
    assert_eq!(path_of(browser.location()), "/home/p");
}

#[test]
fn the_sheet_answers_nothing_while_it_is_away() {
    let (browser, effects) = run(
        knowing_home(),
        vec![typed("/tmp"), Input::GoToSubmit, Input::GoToCancel],
    );
    assert_eq!(browser.go_to(), &GoTo::Closed);
    assert!(effects.is_empty());
}

#[test]
fn going_elsewhere_keeps_home_and_closes_the_sheet() {
    let (browser, _) = run(
        knowing_home(),
        vec![Input::GoToFolder, Input::Go(dir("/tmp")), Input::GoToFolder],
    );
    assert_eq!(browser.go_to(), &asking("/tmp", None, None));
    let (browser, _) = run(browser, vec![typed("~"), Input::GoToSubmit]);
    assert_eq!(
        browser.go_to(),
        &asking("~", None, Some("/home/p")),
        "home survives a visit"
    );
}
