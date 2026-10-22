//! The edge with several tabs: an answer goes back to the tab that asked, a watch belongs to one
//! tab.

use files_core::{Answer, FolderPath, ItemRef, Location, OpPlan, Resolution, Transfer};
use files_fs::FolderEvent;
use files_model::{Change, Effect, Input, OpEvent, TabId, Tabbed};
use files_ui::edge::Heard;

use super::edge::{file, folder, name, next, rig};
use super::edge_ops::until;
use super::support::scratch_folder;

/// The tab ids a window hands out: the first, and the ones a New Tab opens after it.
fn tabs() -> (TabId, TabId) {
    let (tabs, _) = files_model::Tabs::new(Location::Trash);
    let (tabs, _) = tabs.step(Input::NewTab, files_model::Now(std::time::Duration::ZERO));
    (TabId::FIRST, tabs.front_id())
}

#[test]
fn a_listing_goes_back_to_the_tab_that_asked_whichever_is_in_front() {
    let (first, second) = tabs();
    let mut rig = rig();
    rig.lister.set(folder("/p"), Ok(vec![file("a")]));
    rig.lister.set(folder("/q"), Ok(vec![file("b")]));
    let (p, q) = (
        Location::Folder(folder("/p")),
        Location::Folder(folder("/q")),
    );
    rig.edge.run(vec![
        Tabbed::new(second, Effect::List(q.clone())),
        Tabbed::new(first, Effect::List(p.clone())),
    ]);
    let mut heard = vec![next(&mut rig.inbox), next(&mut rig.inbox)];
    heard.sort_by_key(|h| format!("{h:?}"));
    let want = [
        Heard::Tab(first, Input::Listed(p, Ok(vec![file("a")]))),
        Heard::Tab(second, Input::Listed(q, Ok(vec![file("b")]))),
    ];
    for want in want {
        assert!(heard.contains(&want), "{want:?} is not in {heard:?}");
    }
}

#[test]
fn two_tabs_on_one_folder_each_watch_it_and_each_stop_alone() {
    let (first, second) = tabs();
    let mut rig = rig();
    let here = folder("/p");
    let place = Location::Folder(here.clone());
    rig.edge.run(vec![
        Tabbed::new(first, Effect::Watch(place.clone())),
        Tabbed::new(second, Effect::Watch(place.clone())),
    ]);
    assert_eq!(rig.watcher.watched(), vec![here.clone(), here.clone()]);
    rig.lister.set(here.clone(), Ok(vec![file("a")]));
    rig.watcher.push(&here, FolderEvent::Changed(name("a")));
    // Both hear the change, each as its own.
    let heard = [next(&mut rig.inbox), next(&mut rig.inbox)];
    for tab in [first, second] {
        let want = Heard::Tab(
            tab,
            Input::Changed(place.clone(), Change::Updated(file("a"))),
        );
        assert!(heard.contains(&want), "{want:?} is not in {heard:?}");
    }
    rig.edge
        .run(vec![Tabbed::new(first, Effect::Unwatch(place))]);
    assert_eq!(rig.watcher.watched(), vec![here]);
}

#[test]
fn the_effect_that_closes_the_window_is_heard_as_such() {
    let mut rig = rig();
    rig.edge.run_in(TabId::FIRST, vec![Effect::CloseWindow]);
    assert_eq!(next(&mut rig.inbox), Heard::CloseWindow);
}

/// A rename of `from` to `to` in `dir`, asked for by `tab`.
fn rename(tab: TabId, dir: &std::path::Path, from: &str, to: &str) -> Tabbed {
    let effect = Effect::Rename {
        folder: FolderPath::new(dir).unwrap(),
        from: name(from),
        to: name(to),
    };
    Tabbed::new(tab, effect)
}

/// What a tab heard of its jobs, as short words, until `ends` jobs have finished.
fn reports(rig: &mut super::edge::Rig, ends: usize) -> Vec<(TabId, String)> {
    let mut said = Vec::new();
    let mut finished = 0;
    while finished < ends {
        match until(&mut rig.inbox, |h| matches!(h, Heard::Tab(_, Input::Op(_)))) {
            Heard::Tab(tab, Input::Op(OpEvent::Started(_))) => {
                said.push((tab, "started".to_owned()))
            }
            Heard::Tab(tab, Input::Op(OpEvent::Asking(conflict))) => {
                said.push((tab, format!("asks about {}", conflict.item.display())));
            }
            Heard::Tab(tab, Input::Op(OpEvent::Finished(outcome))) => {
                finished += 1;
                said.push((tab, format!("{:?}", outcome.end)));
            }
            Heard::Tab(..)
            | Heard::All(_)
            | Heard::CloseWindow
            | Heard::Told(_)
            | Heard::Announced(_)
            | Heard::Looked(..)
            | Heard::Thumbed(..) => {}
        }
    }
    said
}

#[test]
fn each_report_of_a_job_goes_to_the_tab_that_asked_even_when_jobs_overlap() {
    let (first, second) = tabs();
    let mut rig = rig();
    let dir = scratch_folder();
    // Both tabs queue before either job is done: the first's succeeds, the second's fails.
    rig.edge.run(vec![
        rename(first, dir.path(), "notes.txt", "ideas.txt"),
        rename(second, dir.path(), "nope.txt", "other.txt"),
    ]);
    let want = [
        (first, "started"),
        (first, "Done"),
        (second, "started"),
        (second, "Failed(NotFound)"),
    ];
    let want: Vec<(TabId, String)> = want.into_iter().map(|(t, w)| (t, w.to_owned())).collect();
    assert_eq!(reports(&mut rig, 2), want);
}

#[test]
fn a_question_is_asked_of_the_tab_whose_job_it_belongs_to() {
    let (first, second) = tabs();
    let mut rig = rig();
    let dir = scratch_folder();
    std::fs::write(dir.path().join("Projects").join("notes.txt"), "other").unwrap();
    let copy = OpPlan::Copy(Transfer::new(
        vec![ItemRef::new(
            FolderPath::new(dir.path().join("Projects")).unwrap(),
            name("notes.txt"),
        )],
        FolderPath::new(dir.path()).unwrap(),
    ));
    // The second tab's copy meets a taken name while the first tab's rename waits behind it.
    rig.edge.run(vec![
        Tabbed::new(second, Effect::Run(copy)),
        rename(first, dir.path(), "report.pdf", "final.pdf"),
    ]);
    let asked = until(&mut rig.inbox, |h| {
        matches!(h, Heard::Tab(_, Input::Op(OpEvent::Asking(_))))
    });
    let Heard::Tab(asker, Input::Op(OpEvent::Asking(conflict))) = asked else {
        panic!("not asked");
    };
    assert_eq!((asker, conflict.item), (second, name("notes.txt")));
    rig.edge.run(vec![Tabbed::new(
        second,
        Effect::Answer(Resolution::once(Answer::KeepBoth)),
    )]);
    let said = reports(&mut rig, 2);
    let said: Vec<_> = said
        .iter()
        .map(|(tab, what)| (*tab, what.as_str()))
        .collect();
    assert_eq!(
        said,
        [(second, "Done"), (first, "started"), (first, "Done")]
    );
}
