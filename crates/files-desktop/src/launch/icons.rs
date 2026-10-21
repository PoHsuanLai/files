//! Finding the image file of an app's icon: the `Icon=` value of its desktop entry is either a
//! path or the name of an icon in the person's icon theme.

use std::collections::HashMap;
use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex, PoisonError};

/// Finds an icon in the icon theme by name.
///
/// The one that reads the theme (and so the environment) belongs to the binary; this crate asks
/// it through the trait, and tests use [`FakeIcons`].
pub trait IconLookup: Send + Sync {
    /// The image file of the icon called `name` (`firefox`, `org.gnome.Calculator`), at the size
    /// the lookup was made for; `None` when the theme has none.
    fn find(&self, name: &str) -> Option<PathBuf>;
}

/// No theme at all: no icon is ever found.
#[derive(Debug, Clone, Copy, Default)]
pub struct NoIcons;

impl IconLookup for NoIcons {
    fn find(&self, _name: &str) -> Option<PathBuf> {
        None
    }
}

/// Resolves entries' `Icon=` values to image files and remembers each answer, found or not, for
/// as long as it lives.
pub struct AppIcons {
    lookup: Arc<dyn IconLookup>,
    seen: Mutex<HashMap<String, Option<PathBuf>>>,
}

impl fmt::Debug for AppIcons {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("AppIcons").finish_non_exhaustive()
    }
}

impl AppIcons {
    /// Icons found through `lookup`.
    pub fn new(lookup: Arc<dyn IconLookup>) -> AppIcons {
        AppIcons {
            lookup,
            seen: Mutex::new(HashMap::new()),
        }
    }

    /// The image file for an entry's `Icon=` value: an absolute path as it is, a name as the
    /// theme has it. `None` for no value or no such icon.
    pub fn resolve(&self, icon: Option<&str>) -> Option<PathBuf> {
        let icon = icon?;
        let path = Path::new(icon);
        if path.is_absolute() {
            return Some(path.to_owned());
        }
        let mut seen = self.seen.lock().unwrap_or_else(PoisonError::into_inner);
        seen.entry(icon.to_owned())
            .or_insert_with(|| self.lookup.find(icon))
            .clone()
    }
}

/// A theme that is a table of names to files, and counts the questions it was asked, for tests.
#[cfg(feature = "testing")]
#[derive(Debug, Default)]
pub struct FakeIcons {
    icons: HashMap<String, PathBuf>,
    asked: std::sync::atomic::AtomicUsize,
}

#[cfg(feature = "testing")]
impl FakeIcons {
    /// A theme with no icons.
    pub fn new() -> FakeIcons {
        FakeIcons::default()
    }

    /// The same theme with the icon `name` at `file`.
    pub fn with(mut self, name: &str, file: &str) -> FakeIcons {
        self.icons.insert(name.to_owned(), PathBuf::from(file));
        self
    }

    /// How many times the theme was asked.
    pub fn asked(&self) -> usize {
        self.asked.load(std::sync::atomic::Ordering::Relaxed)
    }
}

#[cfg(feature = "testing")]
impl IconLookup for FakeIcons {
    fn find(&self, name: &str) -> Option<PathBuf> {
        self.asked
            .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
        self.icons.get(name).cloned()
    }
}
