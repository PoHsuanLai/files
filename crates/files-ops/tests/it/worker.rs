//! The queue on its thread: events in order, a conflict answered from outside, cancel, undo.

use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};
use std::time::Duration;

use files_core::{Answer, OpKind, OpPlan, Resolution};
use files_ops::{Event, Finish, Job, Worker};
use files_trash::FakeTrash;

use super::support::{name, rig};

/// Who asked for a job: stands in for the caller's tag.
type Asker = u8;

fn start(rig: &super::support::Rig) -> (Worker<Asker>, Receiver<(Asker, Event)>) {
    let (sender, events) = channel();
    let sender = std::sync::Mutex::new(sender);
    let sink = move |asker: &Asker, event: Event| {
        let _ = sender.lock().unwrap().send((*asker, event));
    };
    let trash = Arc::new(FakeTrash::new(rig.path("queue-bin")));
    (Worker::start(trash, Arc::new(sink)).unwrap(), events)
}

fn next_tagged(events: &Receiver<(Asker, Event)>) -> (Asker, Event) {
    events.recv_timeout(Duration::from_secs(5)).unwrap()
}

fn next(events: &Receiver<(Asker, Event)>) -> Event {
    next_tagged(events).1
}

/// The next `Finished`, skipping progress.
fn finished(events: &Receiver<(Asker, Event)>) -> files_ops::Finished {
    loop {
        if let Event::Finished(done) = next(events) {
            return done;
        }
    }
}

#[test]
fn work_is_reported_started_then_finished_and_can_be_undone() {
    let rig = rig();
    let (worker, events) = start(&rig);
    let plan = OpPlan::NewFolder {
        parent: rig.folder("home"),
        name: name("untitled folder"),
    };
    worker.submit(Job::Run(plan), 1);

    let Event::Started(summary) = next(&events) else {
        panic!("not started first");
    };
    assert_eq!(summary.kind, OpKind::NewFolder);
    let done = finished(&events);
    assert_eq!(done.finish, Finish::Done);
    assert_eq!(done.undo.map(|s| s.kind), Some(OpKind::NewFolder));
    assert!(rig.path("home/untitled folder").is_dir());

    worker.submit(Job::Undo, 1);
    let undone = finished(&events);
    assert_eq!(undone.finish, Finish::Done);
    assert_eq!(undone.undo, None, "nothing left to undo");
    assert!(!rig.exists("home/untitled folder"));
}

#[test]
fn a_conflict_waits_for_the_answer_and_the_work_goes_on() {
    let rig = rig();
    rig.write("home/a.txt", "new");
    rig.write("dest/a.txt", "old");
    let (worker, events) = start(&rig);
    worker.submit(Job::Run(OpPlan::Copy(rig.transfer(&["home/a.txt"]))), 1);

    let conflict = loop {
        if let Event::Asking(conflict) = next(&events) {
            break conflict;
        }
    };
    assert_eq!(conflict.item, name("a.txt"));
    assert_eq!(
        rig.read("dest/a.txt"),
        "old",
        "nothing happens before the answer"
    );

    worker.answer(Resolution::once(Answer::KeepBoth));
    assert_eq!(finished(&events).finish, Finish::Done);
    assert_eq!(rig.names("dest"), ["a 2.txt", "a.txt"]);
}

#[test]
fn cancel_answers_a_waiting_conflict_with_stop() {
    let rig = rig();
    rig.write("home/a.txt", "new");
    rig.write("dest/a.txt", "old");
    let (worker, events) = start(&rig);
    worker.submit(Job::Run(OpPlan::Copy(rig.transfer(&["home/a.txt"]))), 1);
    loop {
        if let Event::Asking(_) = next(&events) {
            break;
        }
    }
    worker.cancel();
    assert_eq!(finished(&events).finish, Finish::Stopped);
    assert_eq!(rig.names("dest"), ["a.txt"]);
}

#[test]
fn work_queued_behind_runs_after_and_a_stale_answer_is_ignored() {
    let rig = rig();
    rig.write("home/a.txt", "a");
    let (worker, events) = start(&rig);
    worker.answer(Resolution::once(Answer::Replace)); // nobody asked
    worker.submit(Job::Run(OpPlan::Move(rig.transfer(&["home/a.txt"]))), 1);
    worker.submit(
        Job::Run(OpPlan::Rename {
            item: rig.item("dest/a.txt"),
            to: name("b.txt"),
        }),
        2,
    );
    // Every event of a job carries the tag it was submitted with, whatever else is queued.
    let mut kinds = Vec::new();
    while kinds.len() < 2 {
        let (asker, event) = next_tagged(&events);
        match event {
            Event::Started(summary) => {
                assert_eq!(asker, if summary.kind == OpKind::Move { 1 } else { 2 })
            }
            Event::Finished(done) => {
                assert_eq!(
                    asker,
                    if done.summary.kind == OpKind::Move {
                        1
                    } else {
                        2
                    }
                );
                kinds.push(done.summary.kind);
            }
            _ => {}
        }
    }
    assert_eq!(kinds, [OpKind::Move, OpKind::Rename]);
    assert_eq!(rig.names("dest"), ["b.txt"]);
}
