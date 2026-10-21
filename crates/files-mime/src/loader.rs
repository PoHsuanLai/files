//! The only code that reads files: the XDG folders, given as an argument.

use std::fs;
use std::io;
use std::path::{Path, PathBuf};

use crate::assoc::{Associations, Installed};
use crate::db::{MimeDb, MimeLayer};
use crate::desktop_id::DesktopId;
use crate::error::LoadError;
use crate::mimeapps::{MimeApps, MimeCache};
use crate::text::data_lines;

/// The largest file read; a bigger one is skipped.
const MAX_BYTES: u64 = 8 * 1024 * 1024;

/// The XDG folders and the desktop's names, filled in by the caller from the environment.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct XdgDirs {
    /// `$XDG_DATA_HOME`.
    pub data_home: PathBuf,
    /// `$XDG_DATA_DIRS`, in order.
    pub data_dirs: Vec<PathBuf>,
    /// `$XDG_CONFIG_HOME`.
    pub config_home: PathBuf,
    /// `$XDG_CONFIG_DIRS`, in order.
    pub config_dirs: Vec<PathBuf>,
    /// `$XDG_CURRENT_DESKTOP`, split at the colons.
    pub current_desktops: Vec<String>,
}

impl XdgDirs {
    /// The data folders, most specific first.
    fn data_search(&self) -> Vec<&Path> {
        std::iter::once(self.data_home.as_path())
            .chain(self.data_dirs.iter().map(PathBuf::as_path))
            .collect()
    }
}

/// A loaded value and how many files were skipped because they could not be read or were too big.
#[derive(Debug, Clone, PartialEq, Eq)]
#[non_exhaustive]
pub struct Loaded<T> {
    /// What was loaded.
    pub value: T,
    /// Files that were there but skipped.
    pub skipped: usize,
}

enum Read {
    Text(String),
    Missing,
    Skipped,
}

fn read_capped(path: &Path) -> Read {
    match fs::metadata(path) {
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Read::Missing,
        Err(_) => return Read::Skipped,
        Ok(meta) if meta.len() > MAX_BYTES => return Read::Skipped,
        Ok(_) => {}
    }
    match fs::read_to_string(path) {
        Ok(text) => Read::Text(text),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Read::Missing,
        Err(_) => Read::Skipped,
    }
}

#[derive(Default)]
struct LayerText {
    globs2: String,
    aliases: String,
    subclasses: String,
    icons: String,
    generic_icons: String,
}

impl LayerText {
    fn read(data_dir: &Path, skipped: &mut usize) -> LayerText {
        let mut file = |name: &str| match read_capped(&data_dir.join("mime").join(name)) {
            Read::Text(text) => text,
            Read::Missing => String::new(),
            Read::Skipped => {
                *skipped += 1;
                String::new()
            }
        };
        LayerText {
            globs2: file("globs2"),
            aliases: file("aliases"),
            subclasses: file("subclasses"),
            icons: file("icons"),
            generic_icons: file("generic-icons"),
        }
    }

    fn layer(&self) -> MimeLayer<'_> {
        MimeLayer {
            globs2: &self.globs2,
            aliases: &self.aliases,
            subclasses: &self.subclasses,
            icons: &self.icons,
            generic_icons: &self.generic_icons,
        }
    }
}

impl MimeDb {
    /// Reads `<data folder>/mime/{globs2,aliases,subclasses,icons,generic-icons}` for each data
    /// folder, `data_home` first. Missing files are fine; unreadable or over-8-MiB ones are
    /// skipped and counted.
    pub fn load(dirs: &XdgDirs) -> Result<Loaded<MimeDb>, LoadError> {
        let mut skipped = 0;
        let texts: Vec<LayerText> = dirs
            .data_search()
            .into_iter()
            .map(|dir| LayerText::read(dir, &mut skipped))
            .collect();
        if texts
            .iter()
            .all(|text| data_lines(&text.globs2).next().is_none())
        {
            return Err(LoadError::NoDatabase);
        }
        let layers: Vec<MimeLayer<'_>> = texts.iter().map(LayerText::layer).collect();
        Ok(Loaded {
            value: MimeDb::from_layers(&layers),
            skipped,
        })
    }
}

/// The `mimeapps.list` files to read, highest precedence first: for the config home and each
/// config folder, then the `applications` folder of the data home and each data folder, the
/// `<desktop>-mimeapps.list` of each current desktop (lower-cased) and then `mimeapps.list`.
pub fn mimeapps_paths(dirs: &XdgDirs) -> Vec<PathBuf> {
    let config = std::iter::once(&dirs.config_home)
        .chain(dirs.config_dirs.iter())
        .cloned();
    let data = std::iter::once(&dirs.data_home)
        .chain(dirs.data_dirs.iter())
        .map(|dir| dir.join("applications"));
    let mut paths = Vec::new();
    for dir in config.chain(data) {
        for desktop in &dirs.current_desktops {
            paths.push(dir.join(format!("{}-mimeapps.list", desktop.to_lowercase())));
        }
        paths.push(dir.join("mimeapps.list"));
    }
    paths
}

impl Associations {
    /// Reads the `mimeapps.list` files (see [`mimeapps_paths`]) and each data folder's
    /// `applications/mimeinfo.cache`; what is missing or unreadable is left out.
    pub fn load(
        dirs: &XdgDirs,
        installed: impl Fn(&DesktopId) -> Installed + Send + Sync + 'static,
    ) -> Associations {
        let files = mimeapps_paths(dirs)
            .iter()
            .filter_map(|path| match read_capped(path) {
                Read::Text(text) => Some(MimeApps::parse(&text)),
                Read::Missing | Read::Skipped => None,
            })
            .collect();
        let caches = dirs
            .data_search()
            .into_iter()
            .filter_map(
                |dir| match read_capped(&dir.join("applications").join("mimeinfo.cache")) {
                    Read::Text(text) => Some(MimeCache::parse(&text)),
                    Read::Missing | Read::Skipped => None,
                },
            )
            .collect();
        Associations::new(files, caches, installed)
    }
}
