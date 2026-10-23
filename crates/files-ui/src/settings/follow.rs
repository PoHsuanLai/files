//! Following the settings file while the window is open, and tidying the Trash when asked to.

use std::sync::Arc;
use std::time::SystemTime;

use ds_blitz::TokioSpawner;
use ds_settings::{ConfigRoot, WatchState};
use files_trash::{THIRTY_DAYS, Trash, sweep};
use tokio::runtime::Handle;
use tokio::sync::watch;

use super::words::Switch;
use super::{FilesDoc, store};
use crate::preferences::Preferences;

/// The person's choices under `root` now, and every change of the file after, on `handle`'s
/// runtime. A file that cannot be watched is read once, and the choices then change only when
/// Files starts again.
///
/// When "remove items from the Trash after 30 days" is on, items of `trash` older than that are
/// erased once now and once each time the file changes with it on.
pub fn follow(
    root: ConfigRoot,
    handle: &Handle,
    trash: Arc<dyn Trash>,
) -> watch::Receiver<Preferences> {
    let mut file = store(root).watch::<FilesDoc>(&TokioSpawner::on(handle.clone()));
    let first = file.current();
    let (send, receive) = watch::channel(first.preferences());
    tidy(handle, &trash, &first);
    if matches!(file.state(), WatchState::Blind { .. }) {
        return receive;
    }
    let runtime = handle.clone();
    handle.spawn(async move {
        while let Some(loaded) = file.changed().await {
            let doc = loaded.value;
            tidy(&runtime, &trash, &doc);
            let next = doc.preferences();
            send.send_if_modified(|current| {
                let changed = *current != next;
                if changed {
                    *current = next;
                }
                changed
            });
            if send.is_closed() {
                break;
            }
        }
    });
    receive
}

/// Erases what has been in `trash` more than 30 days, when `doc` says to, off the async threads.
fn tidy(handle: &Handle, trash: &Arc<dyn Trash>, doc: &FilesDoc) {
    if doc.advanced.remove_old_trash != Switch::On {
        return;
    }
    let trash = Arc::clone(trash);
    // Nobody waits for the answer: a Trash that cannot be read is tried again at the next start.
    handle.spawn_blocking(move || {
        let _ = sweep(&*trash, SystemTime::now(), THIRTY_DAYS);
    });
}
