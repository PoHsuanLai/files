//! The edge's seams for Open With, Go to Folder and Copy as Pathname.

use std::path::PathBuf;

use files_model::{AppPick, Effect, Input, ListFailure, OpenTarget};
use files_ui::OpenAs;
use files_ui::edge::Heard;

use super::edge::{file, folder, name, next, rig};

#[test]
fn files_are_handed_to_the_app_picked() {
    let rig = rig();
    let target = OpenTarget::new(folder("/p"), name("a.txt"));
    let app = AppPick::new("viewer.desktop");
    rig.edge.run(vec![Effect::OpenFilesWith {
        targets: vec![target],
        app: app.clone(),
    }]);
    assert_eq!(
        rig.opener.calls(),
        vec![(vec![PathBuf::from("/p/a.txt")], OpenAs::App(app))]
    );
}

#[test]
fn a_folder_is_checked_on_a_thread_and_the_answer_comes_back() {
    let mut rig = rig();
    let there = folder("/p/there");
    rig.lister.set(there.clone(), Ok(vec![file("x")]));
    rig.edge.run(vec![Effect::CheckFolder(there.clone())]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::FolderChecked(there, Ok(())))
    );

    let gone = folder("/p/gone");
    rig.edge.run(vec![Effect::CheckFolder(gone.clone())]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Input(Input::FolderChecked(gone, Err(ListFailure::NotFound)))
    );
}

#[test]
fn pathnames_with_no_clipboard_to_reach_are_told_in_words() {
    // Outside a window there is no clipboard to write to.
    let mut rig = rig();
    rig.edge.run(vec![Effect::CopyText("/p/a.txt".to_owned())]);
    assert_eq!(
        next(&mut rig.inbox),
        Heard::Told("The pathnames couldn\u{2019}t be copied.".to_owned())
    );
}
