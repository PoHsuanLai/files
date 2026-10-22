//! Making a thumbnail when the shared cache has none: anyview's peek, the same one Quick Look and
//! anyview's own pane look through, asked for a small picture.

use std::fmt;
use std::sync::Arc;

use anyview_core::{FilePath, FormatKind, Input, PixelLen, PixelSize, Resize};
use anyview_fs::OnDisk;
use anyview_image::{Rgba8, resized};
use anyview_peek::{Body, PageLook, PdfPeeked, Peeking, StillSource, look, probe};
use anyview_platform::{ThumbSize, ThumbnailCache};

/// The size every thumbnail is made at (the cache's `normal`): a tile draws it at most 96 px wide,
/// so this stays sharp on a screen of twice the density.
pub const SIZE: ThumbSize = ThumbSize::Normal;

/// The kinds of file anyview can make a picture of: images, a PDF's first page, a video's frame
/// and a recording's cover. Every other kind keeps its icon.
const PICTURED: [FormatKind; 5] = [
    FormatKind::Raster,
    FormatKind::Vector,
    FormatKind::Pdf,
    FormatKind::Video,
    FormatKind::Audio,
];

/// What making a thumbnail came to.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Made {
    /// A picture of the file, already no larger than [`SIZE`]: kept in the shared cache.
    Pixels(Rgba8),
    /// A PDF's first page, already rasterised by the renderer's own page cache as a `data:` or
    /// `file:` source: shown as it is, and not kept again here.
    Page(String),
}

/// Makes the picture of a file the cache has none of. Blocking: it runs on a worker.
pub trait Maker: Send + Sync {
    /// The picture of `file`, or `None` when it is of a kind nothing pictures or can't be read.
    fn make(&self, file: &FilePath) -> Option<Made>;
}

/// The cache as the source of a video's frame: a thumbnail some program made earlier at a size
/// we don't keep.
struct CachedStills(Arc<dyn ThumbnailCache + Send + Sync>);

impl fmt::Debug for CachedStills {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("CachedStills")
    }
}

impl StillSource for CachedStills {
    fn still(&self, source: &Input) -> Option<Rgba8> {
        let path = source.path()?;
        let stamp = source.stamp();
        [ThumbSize::Large, ThumbSize::XLarge]
            .into_iter()
            .find_map(|size| {
                let found = self.0.lookup(path, &stamp, size).ok().flatten()?;
                Rgba8::new(found.size(), found.rgba().to_vec()).ok()
            })
    }
}

/// anyview's peek as the maker: the file is sniffed, and only a kind anyview pictures is looked at.
#[derive(Debug)]
pub struct Anyview {
    peeking: Peeking,
}

impl Anyview {
    /// A maker whose videos fall back on the pictures `cache` holds at sizes other than ours.
    pub fn new(cache: Arc<dyn ThumbnailCache + Send + Sync>) -> Anyview {
        let edge = SIZE.edge();
        let fit = PixelSize {
            width: edge,
            height: edge,
        };
        Anyview {
            peeking: Peeking::pane()
                .with_fit(fit)
                .with_stills(Arc::new(CachedStills(cache))),
        }
    }
}

impl Maker for Anyview {
    fn make(&self, file: &FilePath) -> Option<Made> {
        let probed = probe(file.on_disk()).ok()?;
        if !PICTURED.contains(&probed.sniffed.kind()) {
            return None;
        }
        let peeked = look(probed.input, &self.peeking);
        if let Body::Picture(image) = &peeked.body {
            return Some(Made::Pixels(fitted(&image.picture, SIZE.edge())));
        }
        if let Body::Page(PdfPeeked {
            page: PageLook::Drawn { source, .. },
        }) = peeked.body
        {
            return Some(Made::Page(source));
        }
        None
    }
}

/// `picture` scaled down so its longer side is at most `edge`; a smaller one is kept as it is.
fn fitted(picture: &Rgba8, edge: PixelLen) -> Rgba8 {
    let size = picture.size();
    if size.width.0.max(size.height.0) > edge.0 {
        resized(picture, Resize::LongEdge(edge))
    } else {
        picture.clone()
    }
}
