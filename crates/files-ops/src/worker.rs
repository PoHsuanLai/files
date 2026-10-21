//! The queue: one thread that does plans one after another, reports as it goes, waits for the
//! person's answer to a conflict, and keeps the undo journal.

use std::io;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, RecvTimeoutError, Sender, channel};
use std::time::{Duration, Instant};

use files_core::OpPlan;
use files_core::{Answer, Conflict, Location, Progress, Resolution, Summary};
use files_trash::Trash;

use crate::exec::Executor;
use crate::host::{CancelToken, Flow, Host};
use crate::journal::Journal;
use crate::ran::{Finish, Ran};

/// How often progress is passed on at most.
const PROGRESS_EVERY: Duration = Duration::from_millis(50);

/// How often a wait for an answer looks to see whether the work was cancelled.
const LOOK_EVERY: Duration = Duration::from_millis(100);

/// What the queue is asked to do.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Job {
    /// Do this plan, after the ones before it.
    Run(OpPlan),
    /// Take back the newest piece of work that can be taken back.
    Undo,
}

/// What the queue reports.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub enum Event {
    /// A piece of work began.
    Started(Summary),
    /// How far it has come.
    Progress(Progress),
    /// It is waiting for an answer to this; send it with [`Worker::answer`].
    Asking(Conflict),
    /// It ended.
    Finished(Finished),
}

/// A piece of work that ended.
#[derive(Debug, Clone, PartialEq)]
#[non_exhaustive]
pub struct Finished {
    /// What the work was about.
    pub summary: Summary,
    /// How it ended.
    pub finish: Finish,
    /// The places that now hold something different.
    pub touched: Vec<Location>,
    /// What undoing would take back now, if anything.
    pub undo: Option<Summary>,
}

impl Finished {
    fn of(ran: Ran, undo: Option<Summary>) -> Finished {
        Finished {
            summary: ran.summary,
            finish: ran.finish,
            touched: ran.touched,
            undo,
        }
    }
}

/// Where the queue sends its events. Any `Fn(Event)` that can be shared between threads is one.
pub trait Sink: Send + Sync {
    /// Takes one event; called from the queue's thread.
    fn send(&self, event: Event);
}

impl<F: Fn(Event) + Send + Sync> Sink for F {
    fn send(&self, event: Event) {
        self(event);
    }
}

/// A queue of work on a thread of its own. Dropping it lets the thread finish what it is doing
/// and end.
#[derive(Debug)]
pub struct Worker {
    jobs: Sender<Job>,
    answers: Sender<Resolution>,
    cancel: CancelToken,
}

impl Worker {
    /// Starts the queue's thread. Work uses `trash` for the Trash; events go to `sink`.
    pub fn start(trash: Arc<dyn Trash>, sink: Arc<dyn Sink>) -> io::Result<Worker> {
        let (jobs, job_inbox) = channel();
        let (answers, answer_inbox) = channel();
        let cancel = CancelToken::new();
        let queue = Queue {
            exec: Executor::new(trash),
            journal: Journal::new(),
            sink,
            jobs: job_inbox,
            answers: answer_inbox,
            cancel: cancel.clone(),
        };
        std::thread::Builder::new()
            .name("files-ops".to_owned())
            .spawn(move || queue.work())?;
        Ok(Worker {
            jobs,
            answers,
            cancel,
        })
    }

    /// Puts `job` at the end of the queue.
    pub fn submit(&self, job: Job) {
        // A gone thread means the app is closing.
        let _ = self.jobs.send(job);
    }

    /// Answers the conflict the work is waiting on.
    pub fn answer(&self, resolution: Resolution) {
        let _ = self.answers.send(resolution);
    }

    /// Stops the work in progress as soon as it is safe; what is done stays done. Work queued
    /// behind it still runs.
    pub fn cancel(&self) {
        self.cancel.cancel();
    }
}

/// Everything the thread owns.
struct Queue {
    exec: Executor,
    journal: Journal,
    sink: Arc<dyn Sink>,
    jobs: Receiver<Job>,
    answers: Receiver<Resolution>,
    cancel: CancelToken,
}

impl Queue {
    fn work(mut self) {
        while let Ok(job) = self.jobs.recv() {
            self.cancel.clear();
            // An answer nobody asked for is stale.
            while self.answers.try_recv().is_ok() {}
            self.one(job);
        }
    }

    fn one(&mut self, job: Job) {
        let mut host = QueueHost {
            sink: &*self.sink,
            answers: &self.answers,
            cancel: &self.cancel,
            last: None,
        };
        let ran = match job {
            Job::Run(plan) => {
                host.sink.send(Event::Started(plan.summary()));
                let ran = self.exec.run(&plan, &mut host);
                self.journal.record(&ran);
                ran
            }
            Job::Undo => {
                if let Some(summary) = self.journal.last() {
                    host.sink.send(Event::Started(summary.clone()));
                }
                match self.journal.undo(&self.exec, &mut host) {
                    Some(ran) => ran,
                    None => return,
                }
            }
        };
        let undo = self.journal.last().cloned();
        self.sink.send(Event::Finished(Finished::of(ran, undo)));
    }
}

/// The host of a piece of work on the queue's thread.
struct QueueHost<'a> {
    sink: &'a dyn Sink,
    answers: &'a Receiver<Resolution>,
    cancel: &'a CancelToken,
    last: Option<Instant>,
}

impl Host for QueueHost<'_> {
    fn progress(&mut self, progress: Progress) {
        let due = self.last.is_none_or(|at| at.elapsed() >= PROGRESS_EVERY);
        if due || progress.permille() >= 1000 {
            self.last = Some(Instant::now());
            self.sink.send(Event::Progress(progress));
        }
    }

    fn ask(&mut self, conflict: &Conflict) -> Resolution {
        self.sink.send(Event::Asking(conflict.clone()));
        loop {
            match self.answers.recv_timeout(LOOK_EVERY) {
                Ok(resolution) => return resolution,
                Err(RecvTimeoutError::Timeout) if self.cancel.flow() == Flow::Go => {}
                Err(RecvTimeoutError::Timeout | RecvTimeoutError::Disconnected) => {
                    return Resolution::once(Answer::Stop);
                }
            }
        }
    }

    fn flow(&self) -> Flow {
        self.cancel.flow()
    }
}
