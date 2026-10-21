//! Quick Look's previews: anyview's peek worker, asked for the latest item only.

use std::sync::Arc;

use anyview_core::FilePath;
use anyview_fs::OnDisk;
use anyview_peek::{PeekWorker, Peeking, WorkerConfig};
use files_core::ItemRef;
use tokio::sync::mpsc::UnboundedSender;

use super::Heard;

/// The worker that makes previews, as the pane of anyview makes them: one thread that looks at
/// the latest file asked for, so holding an arrow key peeks at the few files it reaches.
pub(super) fn worker() -> PeekWorker {
    PeekWorker::looking(
        WorkerConfig::default().with_name("files-peek"),
        Peeking::pane(),
    )
}

/// Asks `worker` for the preview of `item`; it comes back to `heard` as [`Heard::Looked`]. An
/// earlier ask that has not started is dropped: its answer would be stale.
pub(super) fn ask(worker: &PeekWorker, item: ItemRef, heard: UnboundedSender<Heard>) {
    let Ok(path) = FilePath::new(item.path()) else {
        return;
    };
    worker.ask(path.on_disk(), move |peeked| {
        // A closed channel means the window is gone.
        let _ = heard.send(Heard::Looked(item, Arc::new(peeked)));
    });
}
