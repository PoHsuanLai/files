//! Where the program keeps what changes as it is used and is not a choice: how each folder is
//! shown. The environment is read once, here, by quire's settings root.

use std::path::PathBuf;
use std::sync::Arc;

use ds_settings::{AppName, ConfigRoot};
use files_fs::{NoPrefs, PrefsStore, TomlPrefs};

/// The directory name under the state folder (`$XDG_STATE_HOME`, else `~/.local/state`).
const APP: AppName = AppName("files");

/// The file the folder views are kept in under `root`; `None` when the root has no state folder
/// (no `$HOME` to start from).
pub fn prefs_file(root: &ConfigRoot) -> Option<PathBuf> {
    root.state_dir(APP).map(|dir| dir.join("folders.toml"))
}

/// The store of folder views for `root`: the file when there is a place for it, else one that
/// remembers nothing (every folder opens with the defaults).
pub fn prefs_store(root: &ConfigRoot) -> Arc<dyn PrefsStore> {
    match prefs_file(root) {
        Some(file) => Arc::new(TomlPrefs::new(file)),
        None => Arc::new(NoPrefs),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_views_live_in_the_state_folder_of_the_root() {
        let root = ConfigRoot::Scratch(PathBuf::from("/scratch"));
        assert_eq!(
            prefs_file(&root),
            Some(PathBuf::from("/scratch/.state/files/folders.toml"))
        );
    }
}
