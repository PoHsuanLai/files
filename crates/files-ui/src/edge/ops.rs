//! Changes to the files: the queue's events as the browser's inputs, and what the Trash lists as
//! rows.

use std::sync::Arc;

use files_core::{Entry, Location, OpFailure, TrashedItem};
use files_model::{Change, End, Input, ListFailure, OpEvent, Outcome};
use files_ops::{Event, Finish, Finished, Sink};
use files_trash::TrashError;
use tokio::sync::mpsc::UnboundedSender;

use super::Heard;

/// A sink that turns the queue's events into what the window hears.
pub(super) fn sink(heard: UnboundedSender<Heard>) -> Arc<dyn Sink> {
    Arc::new(move |event: Event| {
        for said in heard_of(event) {
            // A closed channel means the window is gone.
            let _ = heard.send(said);
        }
    })
}

/// What the window hears of `event`: the report itself and, when work ended, a rescan of every
/// place it changed (the watcher usually says so first; this makes sure the listing catches up).
fn heard_of(event: Event) -> Vec<Heard> {
    match event {
        Event::Started(summary) => vec![op(OpEvent::Started(summary))],
        Event::Progress(progress) => vec![op(OpEvent::Progress(progress))],
        Event::Asking(conflict) => vec![op(OpEvent::Asking(conflict))],
        Event::Finished(done) => finished(done),
        // An event files-ops adds later is not told until this names it.
        _ => Vec::new(),
    }
}

fn op(event: OpEvent) -> Heard {
    Heard::Input(Input::Op(event))
}

fn finished(done: Finished) -> Vec<Heard> {
    let outcome = Outcome::new(done.summary, end_of(done.finish), done.undo);
    let mut heard = vec![op(OpEvent::Finished(outcome))];
    heard.extend(done.touched.into_iter().map(rescan));
    heard
}

fn rescan(place: Location) -> Heard {
    Heard::Input(Input::Changed(place, Change::Rescan))
}

fn end_of(finish: Finish) -> End {
    match finish {
        Finish::Done => End::Done,
        Finish::Stopped => End::Stopped,
        Finish::Failed(failure) => End::Failed(failure),
        // An ending files-ops adds later is a failure until this names it.
        _ => End::Failed(OpFailure::Other),
    }
}

/// The row for a trashed item: named as the Trash stores it (unique there), with the date it was
/// thrown away as its date. `None` for an item whose stored name cannot be read.
pub(super) fn row_of(item: TrashedItem) -> Option<Entry> {
    let name = item.stored_name()?;
    Some(
        Entry::new(name, item.kind.clone())
            .modified_at(item.deleted)
            .in_trash(item),
    )
}

/// The failure a Trash error is to the browser.
pub(super) fn failure_of(err: TrashError) -> ListFailure {
    match err {
        TrashError::NoPermission => ListFailure::NoPermission,
        TrashError::NotFound => ListFailure::NotFound,
        TrashError::Occupied | TrashError::Other => ListFailure::Other,
        // An error files-trash adds later is an ordinary failure until this names it.
        _ => ListFailure::Other,
    }
}
