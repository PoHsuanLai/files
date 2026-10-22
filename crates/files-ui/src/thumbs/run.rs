//! One thumbnail, start to finish: the shared cache first, then making it and keeping it there.

use std::fmt;
use std::sync::Arc;

use anyview_core::FilePath;
use anyview_fs::OnDisk;
use anyview_image::{Rgba8, encode_png};
use anyview_platform::{ThumbPixels, ThumbnailCache};
use ds::prelude::ImageSource;

use super::key::Thumbed;
use super::make::{Made, Maker, SIZE};

/// Finds or makes the thumbnail of a file. Blocking: every call runs on a worker.
pub struct Thumbnailer {
    cache: Arc<dyn ThumbnailCache + Send + Sync>,
    maker: Arc<dyn Maker>,
}

impl fmt::Debug for Thumbnailer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("Thumbnailer")
    }
}

impl Thumbnailer {
    /// A thumbnailer over the shared `cache`, making what it lacks with `maker`.
    pub fn new(cache: Arc<dyn ThumbnailCache + Send + Sync>, maker: Arc<dyn Maker>) -> Thumbnailer {
        Thumbnailer { cache, maker }
    }

    /// The thumbnail of `file` as it is now. One kept for an earlier version of the file (its size
    /// or date differ) is no thumbnail, so a changed file is made again and the new one replaces
    /// the old. A file nothing pictures keeps its icon. A cache that can't be read or written is
    /// no more than a cache that has nothing.
    pub fn thumbnail(&self, file: &FilePath) -> Thumbed {
        let stamp = file.on_disk().stamp();
        let kept = self.cache.lookup(file, &stamp, SIZE).ok().flatten();
        if let Some(pixels) = kept
            && let Ok(picture) = Rgba8::new(pixels.size(), pixels.rgba().to_vec())
        {
            return shown(&picture);
        }
        match self.maker.make(file) {
            Some(Made::Pixels(picture)) => {
                if let Some(pixels) = ThumbPixels::new(picture.size(), picture.bytes().to_vec()) {
                    // Keeping it is a courtesy to the next program; failing to is not a failure.
                    let _ = self.cache.store(file, &stamp, SIZE, &pixels);
                }
                shown(&picture)
            }
            Some(Made::Page(source)) => Thumbed::Picture(ImageSource(source)),
            None => Thumbed::Icon,
        }
    }
}

/// `picture` as the source a tile draws.
fn shown(picture: &Rgba8) -> Thumbed {
    match encode_png(picture) {
        Ok(bytes) => Thumbed::Picture(ImageSource::png(&bytes)),
        Err(_) => Thumbed::Icon,
    }
}
