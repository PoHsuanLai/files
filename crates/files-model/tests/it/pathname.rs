//! Copy as Pathname and Get Info on the folder: what the context menu adds.

use files_core::{FolderPath, ItemRef};
use files_model::{Effect, InfoView, Input, Modifier};

use crate::common::*;

fn click(text: &str) -> Input {
    Input::Click(name(text), Modifier::None)
}

#[test]
fn the_full_paths_of_the_selection_go_to_the_clipboard_one_per_line() {
    // label, inputs, text
    let cases: Vec<(&str, Vec<Input>, Option<&str>)> = vec![
        ("one", vec![click("b")], Some("/home/p/b")),
        (
            "several, in shown order",
            vec![Input::SelectAll],
            Some("/home/p/a\n/home/p/b"),
        ),
        ("nothing selected", vec![], None),
    ];
    for (label, before, want) in cases {
        let (browser, _) = run(loaded(vec![file("b"), file("a")]), before);
        let (_, effects) = browser.step(Input::CopyAsPathname, at(0));
        let want: Vec<Effect> = want
            .map(|text| Effect::CopyText(text.to_owned()))
            .into_iter()
            .collect();
        assert_eq!(effects, want, "{label}");
    }
}

#[test]
fn the_trash_has_no_paths_to_copy() {
    let (browser, _) = files_model::Browser::new(files_core::Location::Trash);
    let (_, effects) = browser.step(Input::CopyAsPathname, at(0));
    assert!(effects.is_empty());
}

#[test]
fn get_info_on_the_folder_ignores_the_selection() {
    let (browser, _) = run(loaded(vec![file("a")]), vec![click("a")]);
    let (browser, effects) = browser.step(Input::GetInfoFolder, at(0));
    let folder = ItemRef::from_path(FolderPath::new("/home/p").unwrap().as_path()).unwrap();
    assert_eq!(effects, vec![Effect::ReadInfo(folder.clone())]);
    assert_eq!(browser.info(), &InfoView::Loading(folder));
}
