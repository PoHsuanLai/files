//! An app's icon file as quire draws it: an image as it is, with no plate under it.

use std::path::Path;

use ds::prelude::{ExternalIcon, IconSize, IconSource};
use ds::style::icon::url::IconUrl;

/// The picture in `file`, drawn `size` across; `None` when the path can't be a `file:` URL (it is
/// not absolute), and the app then shows no picture.
pub(super) fn app_icon(file: &Path, size: IconSize) -> Option<IconSource> {
    let url = IconUrl::file(file).ok()?;
    Some(IconSource::Image(ExternalIcon { url, size }))
}
