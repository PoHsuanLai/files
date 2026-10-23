//! The file icons' resolver, built from what is installed: the Extensions folders and
//! `extensions.toml` are read here (the effects layer), never in a view.

use std::path::Path;
use std::sync::Arc;

use ds_extensions::{Built, ExtensionsFile, FileIcons, Search};
use ds_helpers::mime::MimeDb;
use ds_settings::Store;
use ds_settings::schema::Provided;

/// Where the file icons come from: the Extensions to search and the file type database that names
/// each file's type. The binary gives the real ones; the default has no Extensions and no types.
#[derive(Debug, Clone)]
pub struct IconSources {
    search: Search,
    db: Arc<MimeDb>,
}

impl IconSources {
    /// Icons from the Extensions `search` finds, for the types of `db`.
    pub fn new(search: Search, db: Arc<MimeDb>) -> IconSources {
        IconSources { search, db }
    }

    /// No Extensions and no file types: every file gets the standard icon of the kind it looks
    /// like (a folder, or an unknown file).
    pub fn standard() -> IconSources {
        IconSources::new(
            Search::new(Path::new("/nonexistent/data"), &[]),
            Arc::new(MimeDb::default()),
        )
    }

    /// The file type database.
    pub fn db(&self) -> &Arc<MimeDb> {
        &self.db
    }

    /// The resolver for the pack `provided` (`appearance.file_icons`) names, with the person's own
    /// changes on top. Reads the Extensions folders and the on/off file (in `store`), so it is
    /// called when the choice changes, not per file.
    pub fn build(&self, store: &Store, provided: &Provided) -> Built {
        let off = store.load::<ExtensionsFile>().value.off_ids();
        let found = self.search.scan(&off);
        FileIcons::build(Arc::clone(&self.db), &found, provided)
    }
}

impl Default for IconSources {
    fn default() -> IconSources {
        IconSources::standard()
    }
}
