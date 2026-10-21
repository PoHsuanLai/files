//! Work under way: undo, what is shown from start to finish, questions and how a change ended.

use files_core::{
    Answer, ByteSize, Conflict, ConflictKind, OpFailure, OpKind, Progress, Resolution,
};
use files_model::{Effect, End, Input, OpEvent, Question, Work};

use super::changes::{finished, home_folder, summary, trash_browser};
use crate::common::*;

#[test]
fn undo_works_only_after_a_change_that_can_be_taken_back() {
    let (browser, effects) = loaded(vec![]).step(Input::Undo, at(0));
    assert!(effects.is_empty());
    assert_eq!(browser.can_undo(), files_model::Availability::Unavailable);

    let (browser, _) = browser.step(
        finished(OpKind::Trash, End::Done, Some(summary(OpKind::Trash))),
        at(0),
    );
    assert_eq!(browser.can_undo(), files_model::Availability::Available);
    let (browser, effects) = browser.step(Input::Undo, at(0));
    assert_eq!(effects, vec![Effect::Undo]);

    // The undo ends and there is nothing more to take back.
    let (browser, _) = browser.step(finished(OpKind::Trash, End::Done, None), at(0));
    assert_eq!(browser.can_undo(), files_model::Availability::Unavailable);
}

#[test]
fn work_is_shown_from_start_to_finish() {
    let browser = loaded(vec![]);
    assert_eq!(browser.work(), &Work::Idle);
    let (browser, _) = browser.step(Input::Op(OpEvent::Started(summary(OpKind::Copy))), at(0));
    let progress = Progress::start(4, ByteSize(100)).wrote(25);
    let (browser, _) = browser.step(Input::Op(OpEvent::Progress(progress)), at(0));
    assert_eq!(
        browser.work(),
        &Work::Running {
            summary: summary(OpKind::Copy),
            progress
        }
    );
    let (running_still, effects) = browser.clone().step(Input::CancelWork, at(0));
    assert_eq!(
        (running_still.work() != &Work::Idle, effects),
        (true, vec![Effect::Cancel])
    );

    let (browser, _) = browser.step(finished(OpKind::Copy, End::Done, None), at(0));
    assert_eq!(browser.work(), &Work::Idle);
    let (_, effects) = browser.step(Input::CancelWork, at(0));
    assert!(effects.is_empty(), "nothing to cancel");
}

#[test]
fn a_taken_name_is_a_question_and_the_answer_goes_to_the_worker() {
    let conflict = Conflict::new(name("a.txt"), home_folder(), ConflictKind::Move, 2);
    let (asked, _) = loaded(vec![]).step(Input::Op(OpEvent::Asking(conflict.clone())), at(0));
    assert_eq!(asked.question(), &Question::Conflict(conflict));

    let answer = Resolution::for_rest(Answer::KeepBoth);
    let (answered, effects) = asked.clone().step(Input::Answer(answer), at(0));
    assert_eq!(effects, vec![Effect::Answer(answer)]);
    assert_eq!(answered.question(), &Question::None);

    // Putting the question away answers Stop; a stray answer with no question does nothing.
    let (_, effects) = asked.step(Input::Dismiss, at(0));
    assert_eq!(
        effects,
        vec![Effect::Answer(Resolution::once(Answer::Stop))]
    );
    let (_, stray) = answered.step(Input::Answer(answer), at(0));
    assert!(stray.is_empty());
}

#[test]
fn what_a_change_said_when_it_ended() {
    let table: Vec<(&str, OpKind, End, Vec<Effect>)> = vec![
        (
            "trashed: offer the undo",
            OpKind::Trash,
            End::Done,
            vec![Effect::Announce(summary(OpKind::Trash))],
        ),
        ("copied", OpKind::Copy, End::Done, vec![]),
        ("stopped", OpKind::Move, End::Stopped, vec![]),
        (
            "failed",
            OpKind::Move,
            End::Failed(OpFailure::NoSpace),
            vec![Effect::Tell(OpFailure::NoSpace)],
        ),
    ];
    for (why, kind, end, want) in table {
        let (_, effects) = loaded(vec![]).step(finished(kind, end, None), at(0));
        assert_eq!(effects, want, "{why}");
    }
}

#[test]
fn a_conflict_question_goes_when_its_work_ends() {
    let conflict = Conflict::new(name("a.txt"), home_folder(), ConflictKind::Copy, 0);
    let (asked, _) = loaded(vec![]).step(Input::Op(OpEvent::Asking(conflict)), at(0));
    let (ended, _) = asked.step(finished(OpKind::Copy, End::Stopped, None), at(0));
    assert_eq!(ended.question(), &Question::None);
}

#[test]
fn a_question_survives_going_somewhere_else() {
    let (asked, _) = trash_browser().step(Input::EmptyTrash, at(0));
    let (moved, _) = asked.step(Input::Go(home()), at(0));
    assert_eq!(moved.question(), &Question::EmptyTrash);
}
