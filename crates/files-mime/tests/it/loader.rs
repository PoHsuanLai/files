//! The loader on a scratch folder it creates itself.

use std::fs;
use std::path::{Path, PathBuf};

use files_mime::loader::mimeapps_paths;
use files_mime::{Associations, DesktopId, Installed, LoadError, MimeDb, NameGuess, XdgDirs};

use crate::{mt, name};

/// A scratch folder that removes itself.
struct Scratch(PathBuf);

impl Scratch {
    fn new(tag: &str) -> Scratch {
        let dir = std::env::temp_dir().join(format!("files-mime-{tag}-{}", std::process::id()));
        let _ = fs::remove_dir_all(&dir);
        fs::create_dir_all(&dir).unwrap();
        Scratch(dir)
    }

    fn write(&self, rel: &str, text: &str) {
        let path = self.0.join(rel);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(path, text).unwrap();
    }

    fn path(&self, rel: &str) -> PathBuf {
        self.0.join(rel)
    }
}

impl Drop for Scratch {
    fn drop(&mut self) {
        let _ = fs::remove_dir_all(&self.0);
    }
}

fn dirs(root: &Scratch) -> XdgDirs {
    XdgDirs {
        data_home: root.path("home/share"),
        data_dirs: vec![root.path("usr/share")],
        config_home: root.path("home/config"),
        config_dirs: vec![root.path("etc")],
        current_desktops: vec!["KDE".to_owned(), "Sway".to_owned()],
    }
}

#[test]
fn paths_follow_the_spec_order_with_lower_cased_desktops() {
    let root = Scratch::new("paths");
    let got: Vec<PathBuf> = mimeapps_paths(&dirs(&root));
    let rel: Vec<&Path> = got.iter().map(|p| p.strip_prefix(&root.0).unwrap()).collect();
    let want = [
        "home/config/kde-mimeapps.list",
        "home/config/sway-mimeapps.list",
        "home/config/mimeapps.list",
        "etc/kde-mimeapps.list",
        "etc/sway-mimeapps.list",
        "etc/mimeapps.list",
        "home/share/applications/kde-mimeapps.list",
        "home/share/applications/sway-mimeapps.list",
        "home/share/applications/mimeapps.list",
        "usr/share/applications/kde-mimeapps.list",
        "usr/share/applications/sway-mimeapps.list",
        "usr/share/applications/mimeapps.list",
    ];
    assert_eq!(rel, want.iter().map(Path::new).collect::<Vec<_>>());
}

#[test]
fn the_database_loads_across_folders_and_counts_skips() {
    let root = Scratch::new("db");
    root.write("usr/share/mime/globs2", "50:text/plain:*.txt\n50:image/png:*.png\n");
    root.write("home/share/mime/globs2", "0:image/png:__NOGLOBS__\n");
    // A folder where a file should be cannot be read as text.
    fs::create_dir_all(root.path("home/share/mime/aliases")).unwrap();
    let loaded = MimeDb::load(&dirs(&root)).unwrap();
    assert_eq!(loaded.skipped, 1);
    assert_eq!(loaded.value.by_name(&name("a.txt")), NameGuess::Type(mt("text/plain")));
    assert_eq!(loaded.value.by_name(&name("a.png")), NameGuess::Unknown);
}

#[test]
fn no_globs_anywhere_is_no_database() {
    let root = Scratch::new("none");
    root.write("usr/share/mime/aliases", "a/b c/d\n");
    assert!(matches!(MimeDb::load(&dirs(&root)), Err(LoadError::NoDatabase)));
}

#[test]
fn associations_load_files_and_caches_and_skip_uninstalled() {
    let root = Scratch::new("apps");
    root.write("home/config/kde-mimeapps.list", "[Default Applications]\ntext/plain=kate.desktop\n");
    root.write("etc/mimeapps.list", "[Default Applications]\ntext/plain=gedit.desktop\n");
    root.write("usr/share/applications/mimeinfo.cache", "[MIME Cache]\ntext/plain=cached.desktop;\n");
    let installed = |id: &DesktopId| {
        if id.as_str() == "kate.desktop" { Installed::No } else { Installed::Yes }
    };
    let assoc = Associations::load(&dirs(&root), installed);
    let db = MimeDb::default();
    let want = ["gedit.desktop", "cached.desktop"].map(|t| DesktopId::new(t).unwrap());
    assert_eq!(assoc.apps_for(&mt("text/plain"), &db)[..2], want);
}
