//! Thumbnails for the icon view, made off the window's thread.
//!
//! A tile that is drawn wants its thumbnail ([`Thumbnails::want`]) and one that scrolls away
//! stops wanting it ([`Thumbnails::unwant`]). A few threads take the wishes, newest first, and
//! answer as [`Heard::Thumbed`] only for a tile that still wants the answer; the work done for a
//! tile that left is still kept in the shared cache. Nothing here waits on the window.

use std::panic::{AssertUnwindSafe, catch_unwind};
use std::sync::{Arc, Condvar, Mutex, MutexGuard, PoisonError};

use anyview_core::{FilePath, FileStamp};
use anyview_platform::{PlatformError, ThumbPixels, ThumbSize, ThumbnailCache};
use tokio::sync::mpsc::UnboundedSender;

use super::{Edge, Heard};
use crate::thumbs::{Delivery, Queue, ThumbKey, Thumbed, Thumbnailer};

/// How many thumbnails are made at once: enough to keep a screenful coming, few enough that
/// scrolling a folder of pictures leaves the machine free for typing.
const WORKERS: usize = 2;

/// The shared cache of a desktop that has none: nothing is found and nothing is kept, so every
/// thumbnail is made each time the window needs one.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoThumbnails;

impl ThumbnailCache for NoThumbnails {
    fn lookup(
        &self,
        _file: &FilePath,
        _stamp: &FileStamp,
        _size: ThumbSize,
    ) -> Result<Option<ThumbPixels>, PlatformError> {
        Ok(None)
    }

    fn store(
        &self,
        _file: &FilePath,
        _stamp: &FileStamp,
        _size: ThumbSize,
        _pixels: &ThumbPixels,
    ) -> Result<(), PlatformError> {
        Ok(())
    }
}

/// The wishes and whether the edge is gone.
#[derive(Default)]
struct State {
    queue: Queue,
    ended: bool,
}

/// What the threads share.
#[derive(Default)]
struct Shared {
    state: Mutex<State>,
    ready: Condvar,
}

impl Shared {
    fn lock(&self) -> MutexGuard<'_, State> {
        self.state.lock().unwrap_or_else(PoisonError::into_inner)
    }
}

/// The threads that make thumbnails.
pub(super) struct Thumbnails {
    shared: Arc<Shared>,
}

impl Thumbnails {
    /// Starts the threads over `thumbnailer`; their answers go to `heard`. If the system would
    /// not start them, no tile gets a picture.
    pub(super) fn start(thumbnailer: Thumbnailer, heard: UnboundedSender<Heard>) -> Thumbnails {
        let shared = Arc::new(Shared::default());
        let thumbnailer = Arc::new(thumbnailer);
        for _ in 0..WORKERS {
            let (shared, thumbnailer, heard) =
                (Arc::clone(&shared), Arc::clone(&thumbnailer), heard.clone());
            let _ = Edge::work("files-thumbs", move || {
                serve(&shared, &thumbnailer, &heard);
            });
        }
        Thumbnails { shared }
    }

    /// A tile that is drawn wants `key`.
    pub(super) fn want(&self, key: ThumbKey) {
        self.shared.lock().queue.want(key);
        self.shared.ready.notify_one();
    }

    /// The tile that wanted `key` was scrolled away or replaced.
    pub(super) fn unwant(&self, key: &ThumbKey) {
        self.shared.lock().queue.unwant(key);
    }
}

impl Drop for Thumbnails {
    fn drop(&mut self) {
        self.shared.lock().ended = true;
        self.shared.ready.notify_all();
    }
}

/// Takes the newest wish, makes it and answers, until the edge is dropped.
fn serve(shared: &Shared, thumbnailer: &Thumbnailer, heard: &UnboundedSender<Heard>) {
    loop {
        let key = {
            let mut state = shared.lock();
            loop {
                if state.ended {
                    return;
                }
                if let Some(key) = state.queue.take() {
                    break key;
                }
                state = shared
                    .ready
                    .wait(state)
                    .unwrap_or_else(PoisonError::into_inner);
            }
        };
        let made = FilePath::new(key.item.path()).map(|file| {
            // A decoder that falls over on one hostile file costs that file its picture only.
            catch_unwind(AssertUnwindSafe(|| thumbnailer.thumbnail(&file))).unwrap_or(Thumbed::Icon)
        });
        let thumbed = made.unwrap_or(Thumbed::Icon);
        let delivery = shared.lock().queue.finish(&key);
        if delivery == Delivery::Deliver {
            // A closed channel means the window is gone.
            let _ = heard.send(Heard::Thumbed(key, thumbed));
        }
    }
}
