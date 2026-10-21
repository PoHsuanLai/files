//! Changes under way with several tabs: every report of a job goes to the tab that asked for it.

use files_core::{
    Answer, ByteSize, Conflict, ConflictKind, FileName, FolderPath, OpFailure, OpKind, Progress,
    Resolution, Summary,
};
use files_model::{
    Availability, Effect, End, Input, OpEvent, Outcome, Question, TabId, Tabbed, Tabs, Work,
};

use crate::common::{at, home, name};

/// Two tabs on the home folder; the second is in front.
fn two() -> (Tabs, TabId, TabId) {
    let (tabs, _) = Tabs::new(home());
    let (tabs, _) = tabs.step(Input::NewTab, at(0));
    let second = tabs.front_id();
    (tabs, TabId::FIRST, second)
}

fn summary(kind: OpKind) -> Summary {
    Summary::new(kind, 1, Some(name("x")))
}

fn conflict(text: &str) -> Conflict {
    let folder = FolderPath::new("/home/p").unwrap();
    Conflict::new(name(text), folder, ConflictKind::Copy, 0)
}

fn finished(kind: OpKind, end: End, undo: Option<OpKind>) -> OpEvent {
    OpEvent::Finished(Outcome::new(summary(kind), end, undo.map(summary)))
}

/// What a tab shows of the change under way: its kind, and the name it is asking about.
type Shown = (Option<OpKind>, Option<FileName>);

/// One report as the queue sends it, tagged with its job's tab, and what both tabs show after.
type Report<'a> = (&'a str, TabId, OpEvent, Shown, Shown);

/// What a tab shows of the change under way: its kind, and the name it is asking about.
fn shown(tabs: &Tabs, tab: TabId) -> (Option<OpKind>, Option<FileName>) {
    let browser = tabs.get(tab).unwrap().browser();
    let kind = match browser.work() {
        Work::Running { summary, .. } => Some(summary.kind),
        Work::Idle | _ => None,
    };
    let asking = match browser.question() {
        Question::Conflict(conflict) => Some(conflict.item.clone()),
        Question::None | Question::EmptyTrash | Question::DeleteForGood(_) | _ => None,
    };
    (kind, asking)
}

#[test]
fn overlapping_jobs_of_two_tabs_each_report_to_their_own_tab() {
    use OpKind::{Copy, Move};
    let (mut tabs, a, b) = two();
    let progress = Progress::start(2, ByteSize(10)).wrote(5);
    // The queue runs A's job, then B's; B asked for its job while A's was running. Each row is one
    // report as the queue sends it, tagged with the tab of its job, and what both tabs show after.
    let rows: Vec<Report> = vec![
        (
            "A starts",
            a,
            OpEvent::Started(summary(Copy)),
            (Some(Copy), None),
            (None, None),
        ),
        (
            "A progresses",
            a,
            OpEvent::Progress(progress),
            (Some(Copy), None),
            (None, None),
        ),
        (
            "A asks, and only A shows the question",
            a,
            OpEvent::Asking(conflict("a.txt")),
            (Some(Copy), Some(name("a.txt"))),
            (None, None),
        ),
        (
            "A ends; B has not started",
            a,
            finished(Copy, End::Done, None),
            (None, None),
            (None, None),
        ),
        (
            "B starts",
            b,
            OpEvent::Started(summary(Move)),
            (None, None),
            (Some(Move), None),
        ),
        (
            "B asks, and only B shows the question",
            b,
            OpEvent::Asking(conflict("b.txt")),
            (None, None),
            (Some(Move), Some(name("b.txt"))),
        ),
        (
            "B ends",
            b,
            finished(Move, End::Done, None),
            (None, None),
            (None, None),
        ),
    ];
    for (why, tab, event, want_a, want_b) in rows {
        let (next, _) = tabs.heard(tab, Input::Op(event), at(0));
        tabs = next;
        assert_eq!(
            (shown(&tabs, a), shown(&tabs, b)),
            (want_a, want_b),
            "{why}"
        );
    }
}

#[test]
fn what_a_closed_tabs_job_reports_is_dropped_but_its_question_is_answered_stop() {
    let (tabs, a, b) = two();
    // B is in front; A closes while its job runs.
    let (tabs, _) = tabs.step(Input::ShowTab(a), at(0));
    let (tabs, _) = tabs.step(Input::CloseTab, at(0));
    assert!(tabs.get(a).is_none());
    assert_eq!(tabs.front_id(), b);

    let stop = Effect::Answer(Resolution::once(Answer::Stop));
    let table: Vec<(&str, OpEvent, Vec<Tabbed>)> = vec![
        ("started", OpEvent::Started(summary(OpKind::Copy)), vec![]),
        (
            "progress",
            OpEvent::Progress(Progress::start(1, ByteSize(1))),
            vec![],
        ),
        (
            "a question is answered Stop, or the queue would wait for ever",
            OpEvent::Asking(conflict("a.txt")),
            vec![Tabbed::new(a, stop)],
        ),
        ("done", finished(OpKind::Copy, End::Done, None), vec![]),
        (
            "stopped",
            finished(OpKind::Copy, End::Stopped, None),
            vec![],
        ),
        (
            "a failure is told in the front tab",
            finished(OpKind::Copy, End::Failed(OpFailure::NoSpace), None),
            vec![Tabbed::new(b, Effect::Tell(OpFailure::NoSpace))],
        ),
    ];
    for (why, event, want) in table {
        let (after, effects) = tabs.clone().heard(a, Input::Op(event), at(0));
        assert_eq!(effects, want, "{why}");
        assert_eq!(
            shown(&after, b),
            (None, None),
            "{why}: the other tab shows nothing"
        );
    }
}

#[test]
fn a_closed_tabs_finished_job_still_updates_what_undo_would_take_back() {
    let (tabs, a, b) = two();
    let (tabs, _) = tabs.step(Input::ShowTab(a), at(0));
    let (tabs, _) = tabs.step(Input::CloseTab, at(0));
    assert_eq!(tabs.front().can_undo(), Availability::Unavailable);
    let done = finished(OpKind::Trash, End::Done, Some(OpKind::Trash));
    let (tabs, _) = tabs.heard(a, Input::Op(done), at(0));
    assert_eq!(
        tabs.get(b).unwrap().browser().can_undo(),
        Availability::Available
    );
}
