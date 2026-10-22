//! Where the thumbnails are shared: the cache of the freedesktop.org thumbnail specification,
//! under the person's cache folder. The environment is read once, here, by anyview's `Env`.

use std::sync::Arc;

use anyview_platform::portable::FreedesktopThumbnails;
use anyview_platform::{Env, ThumbnailCache};

/// The cache every program of the desktop shares, `~/.cache/thumbnails`.
pub fn shared() -> Arc<dyn ThumbnailCache + Send + Sync> {
    Arc::new(FreedesktopThumbnails::new(&Env::from_process()))
}
