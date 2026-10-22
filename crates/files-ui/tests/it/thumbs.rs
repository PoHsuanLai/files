//! Thumbnails: found in the shared cache or made and kept there, remade for a changed file, left
//! as icons for what nothing pictures, and not delivered for a tile that scrolled away; the
//! previews kept by item; and the edge's workers end to end.

use std::path::Path;
use std::sync::{Arc, Mutex, PoisonError};
use std::time::UNIX_EPOCH;

use anyview_core::{FilePath, PixelLen, PixelSize};
use anyview_fs::OnDisk;
use anyview_image::{Rgba8, encode_png};
use anyview_platform::testing::FakeThumbnails;
use anyview_platform::{ThumbPixels, ThumbnailCache};
use ds::prelude::ImageSource;
use files_core::{FileName, FolderPath, ItemRef};
use files_ui::edge::{Edge, Heard, Seams};
use files_ui::thumbs::{
    Delivery, Made, Maker, Queue, SIZE, ThumbKey, Thumbed, Thumbnailer, Thumbs, Version,
};

use super::edge::next;
use super::support::{RecordingOpener, scratch_folder};

fn pixel(shade: u8) -> Rgba8 {
    let size = PixelSize {
        width: PixelLen(1),
        height: PixelLen(1),
    };
    Rgba8::new(size, vec![shade, shade, shade, 255]).unwrap()
}

/// A maker that pictures every file but `.txt` ones, a different shade each time, and counts.
#[derive(Default)]
struct CountingMaker {
    made: Mutex<Vec<String>>,
    page: Option<String>,
}

impl CountingMaker {
    fn count(&self) -> usize {
        self.made
            .lock()
            .unwrap_or_else(PoisonError::into_inner)
            .len()
    }
}

impl Maker for CountingMaker {
    fn make(&self, file: &FilePath) -> Option<Made> {
        let mut made = self.made.lock().unwrap_or_else(PoisonError::into_inner);
        made.push(file.as_path().display().to_string());
        if file.as_path().extension().is_some_and(|ext| ext == "txt") {
            return None;
        }
        if let Some(page) = &self.page {
            return Some(Made::Page(page.clone()));
        }
        Some(Made::Pixels(pixel(u8::try_from(made.len()).unwrap())))
    }
}

fn source_of(shade: u8) -> Thumbed {
    Thumbed::Picture(ImageSource::png(&encode_png(&pixel(shade)).unwrap()))
}

struct Rig {
    cache: FakeThumbnails,
    maker: Arc<CountingMaker>,
    thumbnailer: Thumbnailer,
}

fn rig(page: Option<&str>) -> Rig {
    let cache = FakeThumbnails::default();
    let maker = Arc::new(CountingMaker {
        page: page.map(str::to_owned),
        ..CountingMaker::default()
    });
    let thumbnailer = Thumbnailer::new(
        Arc::new(cache.clone()),
        Arc::clone(&maker) as Arc<dyn Maker>,
    );
    Rig {
        cache,
        maker,
        thumbnailer,
    }
}

fn file_at(dir: &Path, name: &str, text: &str) -> FilePath {
    std::fs::write(dir.join(name), text).unwrap();
    FilePath::new(dir.join(name)).unwrap()
}

#[test]
fn a_thumbnail_is_found_made_and_kept_or_remade_for_a_changed_file() {
    let dir = tempfile::tempdir().unwrap();
    let rig = rig(None);
    let picture = file_at(dir.path(), "cat.png", "one");

    // A miss is made and kept for every program.
    assert_eq!(rig.thumbnailer.thumbnail(&picture), source_of(1));
    assert_eq!((rig.maker.count(), rig.cache.len()), (1, 1));

    // A hit is read from the cache: nothing is made again.
    assert_eq!(rig.thumbnailer.thumbnail(&picture), source_of(1));
    assert_eq!(rig.maker.count(), 1);

    // The file changed on disk: the kept one is for another version, so it is made again, and the
    // new one replaces the old.
    let picture = file_at(dir.path(), "cat.png", "a longer text");
    assert_eq!(rig.thumbnailer.thumbnail(&picture), source_of(2));
    assert_eq!((rig.maker.count(), rig.cache.len()), (2, 1));

    // A thumbnail some other program kept, at this file's version, is used as it is.
    let other = file_at(dir.path(), "dog.png", "bark");
    let stamp = other.on_disk().stamp();
    let kept = ThumbPixels::new(pixel(9).size(), pixel(9).bytes().to_vec()).unwrap();
    rig.cache.store(&other, &stamp, SIZE, &kept).unwrap();
    assert_eq!(rig.thumbnailer.thumbnail(&other), source_of(9));
    assert_eq!(rig.maker.count(), 2);
}

#[test]
fn what_nothing_pictures_keeps_its_icon_and_a_pdf_page_is_shown_as_it_is() {
    let dir = tempfile::tempdir().unwrap();
    let plain = rig(None);
    let notes = file_at(dir.path(), "notes.txt", "hello");
    assert_eq!(plain.thumbnailer.thumbnail(&notes), Thumbed::Icon);
    assert!(plain.cache.is_empty(), "nothing is kept for an icon");

    let paged = rig(Some("file:///cache/page.png"));
    let report = file_at(dir.path(), "report.pdf", "%PDF");
    assert_eq!(
        paged.thumbnailer.thumbnail(&report),
        Thumbed::Picture(ImageSource("file:///cache/page.png".to_owned()))
    );
    assert!(paged.cache.is_empty(), "the renderer keeps its own pages");
}

fn key(name: &str, len: u64) -> ThumbKey {
    ThumbKey {
        item: ItemRef::new(FolderPath::new("/p").unwrap(), FileName::new(name).unwrap()),
        version: Version {
            len,
            modified: Some(UNIX_EPOCH),
        },
    }
}

#[test]
fn wishes_are_made_newest_first_and_an_answer_for_a_tile_that_left_is_dropped() {
    let mut queue = Queue::default();
    let (a, b, c) = (key("a", 1), key("b", 1), key("c", 1));
    for wish in [a.clone(), b.clone(), c.clone(), b.clone()] {
        queue.want(wish);
    }
    assert_eq!(queue.len(), 3, "wanting twice is one wish");

    // The newest is made first; one being made is not taken again.
    assert_eq!(queue.take(), Some(c.clone()));
    assert_eq!(queue.take(), Some(b.clone()));

    // c is scrolled away while it is being made: its answer is dropped. b's tile stays.
    queue.unwant(&c);
    assert_eq!(queue.finish(&c), Delivery::Drop);
    assert_eq!(queue.finish(&b), Delivery::Deliver);

    // a was scrolled away before it was started: it is never taken.
    queue.unwant(&a);
    assert_eq!(queue.take(), None);
    assert!(queue.is_empty());

    // A changed file is another wish, and an answer for the old version is not this one's.
    queue.want(key("a", 1));
    queue.want(key("a", 2));
    assert_eq!(queue.take(), Some(key("a", 2)));
    queue.unwant(&key("a", 1));
    assert_eq!(queue.finish(&key("a", 1)), Delivery::Drop);
}

#[test]
fn the_window_keeps_the_newest_answers_and_forgets_the_oldest() {
    let mut shelf = Thumbs::default();
    assert!(shelf.is_empty());
    shelf.put(key("a", 1), Thumbed::Icon);
    assert_eq!(shelf.get(&key("a", 1)), Some(&Thumbed::Icon));
    assert_eq!(
        shelf.get(&key("a", 2)),
        None,
        "another version has no answer"
    );
    for n in 0..600 {
        shelf.put(key("many", n), Thumbed::Icon);
    }
    assert_eq!(shelf.len(), 512);
    assert_eq!(shelf.get(&key("a", 1)), None, "the oldest went first");
    assert_eq!(shelf.get(&key("many", 599)), Some(&Thumbed::Icon));
}

/// The key of the file `name` in `dir` as the listing would give it.
fn key_on_disk(dir: &Path, name: &str) -> ThumbKey {
    let meta = std::fs::metadata(dir.join(name)).unwrap();
    ThumbKey {
        item: ItemRef::new(FolderPath::new(dir).unwrap(), FileName::new(name).unwrap()),
        version: Version {
            len: meta.len(),
            modified: meta.modified().ok(),
        },
    }
}

#[test]
fn the_edge_makes_a_picture_of_a_real_image_keeps_it_and_leaves_text_an_icon() {
    let dir = scratch_folder();
    let image = encode_png(&pixel(200)).unwrap();
    std::fs::write(dir.path().join("pic.png"), image).unwrap();
    let cache = FakeThumbnails::default();
    let seams =
        Seams::disk(Arc::new(RecordingOpener::default())).with_thumbnails(Arc::new(cache.clone()));
    let (edge, mut inbox) = Edge::new(seams);

    let (pic, notes) = (
        key_on_disk(dir.path(), "pic.png"),
        key_on_disk(dir.path(), "notes.txt"),
    );
    edge.want_thumbnail(pic.clone());
    edge.want_thumbnail(notes.clone());
    let mut heard = Vec::new();
    while heard.len() < 2 {
        match next(&mut inbox) {
            Heard::Thumbed(key, thumbed) => heard.push((key, thumbed)),
            Heard::Tab(..)
            | Heard::All(_)
            | Heard::CloseWindow
            | Heard::Told(_)
            | Heard::Announced(_)
            | Heard::Looked(..) => {}
        }
    }
    for (key, thumbed) in heard {
        if key == pic {
            assert!(
                matches!(thumbed, Thumbed::Picture(_)),
                "no picture of pic.png"
            );
        } else {
            assert_eq!(key, notes);
            assert_eq!(thumbed, Thumbed::Icon);
        }
    }
    assert_eq!(cache.len(), 1, "the picture is kept in the shared cache");
}
