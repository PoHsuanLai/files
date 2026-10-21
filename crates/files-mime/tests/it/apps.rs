//! Parsing `mimeapps.list` and `mimeinfo.cache`, and the default and Open With resolution.

use files_mime::{
    Associations, DesktopId, DesktopIdError, Installed, MimeApps, MimeCache, MimeDb, MimeLayer,
};

use crate::mt;

fn id(text: &str) -> DesktopId {
    DesktopId::new(text).unwrap()
}

fn ids(texts: &[&str]) -> Vec<DesktopId> {
    texts.iter().map(|t| id(t)).collect()
}

fn assoc(files: &[&str], caches: &[&str], missing: &'static [&'static str]) -> Associations {
    Associations::new(
        files.iter().map(|f| MimeApps::parse(f)).collect(),
        caches.iter().map(|c| MimeCache::parse(c)).collect(),
        move |app| {
            if missing.contains(&app.as_str()) {
                Installed::No
            } else {
                Installed::Yes
            }
        },
    )
}

fn db() -> MimeDb {
    let subclasses = "application/x-ruby application/x-shellscript\n\
                      application/x-shellscript text/plain\n";
    MimeDb::from_layers(&[MimeLayer {
        subclasses,
        ..MimeLayer::default()
    }])
}

#[test]
fn desktop_ids_are_checked() {
    let rows = [
        ("org.gnome.Nautilus.desktop", Ok(())),
        ("kde4-foo.desktop", Ok(())),
        ("foo", Err(DesktopIdError::Suffix)),
        (".desktop", Err(DesktopIdError::Empty)),
        ("a/b.desktop", Err(DesktopIdError::Shape)),
        ("a b.desktop", Err(DesktopIdError::Shape)),
    ];
    for (text, want) in rows {
        assert_eq!(DesktopId::new(text).map(|_| ()), want, "{text}");
    }
}

#[test]
fn mimeapps_list_groups_are_read_and_the_rest_skipped() {
    let text = "# c\n[Default Applications]\ntext/plain=a.desktop;b.desktop;\nbad key=x.desktop\n\
                [Added Associations]\ntext/plain=c.desktop;notanapp;\n\
                [Removed Associations]\ntext/plain=d.desktop\n[Other]\ntext/plain=e.desktop\n";
    let apps = MimeApps::parse(text);
    let plain = mt("text/plain");
    assert_eq!(apps.defaults_for(&plain), ids(&["a.desktop", "b.desktop"]));
    assert_eq!(apps.added_for(&plain), ids(&["c.desktop"]));
    assert_eq!(apps.removed_for(&plain), ids(&["d.desktop"]));
    assert!(apps.defaults_for(&mt("image/png")).is_empty());
}

#[test]
fn mimeinfo_cache_reads_only_its_group() {
    let text = "[MIME Cache]\ntext/plain=a.desktop;b.desktop;\nimage/png=c.desktop;\n\
                [Other]\ntext/plain=z.desktop;\n";
    let cache = MimeCache::parse(text);
    assert_eq!(
        cache.apps_for(&mt("text/plain")),
        ids(&["a.desktop", "b.desktop"])
    );
    assert_eq!(cache.apps_for(&mt("image/png")), ids(&["c.desktop"]));
    assert!(cache.apps_for(&mt("video/mp4")).is_empty());
}

const HIGH: &str = "[Default Applications]\ntext/plain=gone.desktop;editor.desktop\n\
                    [Added Associations]\ntext/plain=extra.desktop;\n\
                    [Removed Associations]\ntext/plain=nope.desktop;\n";
const LOW: &str = "[Default Applications]\ntext/plain=other.desktop\n\
                   [Added Associations]\ntext/plain=nope.desktop;low.desktop\n";
const CACHE: &str = "[MIME Cache]\ntext/plain=cacheapp.desktop;editor.desktop;nope.desktop;\n";

#[test]
fn the_default_skips_uninstalled_apps_and_the_first_file_wins() {
    let assoc = assoc(&[HIGH, LOW], &[CACHE], &["gone.desktop"]);
    assert_eq!(
        assoc.default_for(&mt("text/plain"), &db()),
        Some(id("editor.desktop"))
    );
}

#[test]
fn open_with_orders_defaults_added_then_cache_without_repeats_or_removed() {
    let assoc = assoc(&[HIGH, LOW], &[CACHE], &["gone.desktop"]);
    let want = ids(&[
        "editor.desktop",
        "other.desktop",
        "extra.desktop",
        "low.desktop",
        "cacheapp.desktop",
    ]);
    assert_eq!(assoc.apps_for(&mt("text/plain"), &db()), want);
}

#[test]
fn a_removal_only_reaches_downward_and_a_default_only_its_own_file() {
    let added = "[Added Associations]\ntext/plain=x.desktop\n";
    let removed = "[Removed Associations]\ntext/plain=x.desktop\n";
    let below = assoc(&[added, removed], &[], &[]);
    assert_eq!(
        below.apps_for(&mt("text/plain"), &db()),
        ids(&["x.desktop"])
    );
    let above = assoc(&[removed, added], &[], &[]);
    assert!(above.apps_for(&mt("text/plain"), &db()).is_empty());

    let own = "[Default Applications]\ntext/plain=a.desktop\n[Removed Associations]\ntext/plain=a.desktop\n";
    let next = "[Default Applications]\ntext/plain=b.desktop\n";
    let assoc = assoc(&[own, next], &[], &[]);
    assert_eq!(
        assoc.default_for(&mt("text/plain"), &db()),
        Some(id("b.desktop"))
    );
}

#[test]
fn an_uninstalled_default_falls_through_to_the_cache() {
    let file = "[Default Applications]\ntext/plain=gone.desktop\n";
    let cache = "[MIME Cache]\ntext/plain=cached.desktop\n";
    let assoc = assoc(&[file], &[cache], &["gone.desktop"]);
    assert_eq!(
        assoc.default_for(&mt("text/plain"), &db()),
        Some(id("cached.desktop"))
    );
}

#[test]
fn desktop_specific_files_listed_first_beat_general_ones() {
    let kde = "[Default Applications]\ntext/plain=kate.desktop\n";
    let general = "[Default Applications]\ntext/plain=gedit.desktop\n";
    let assoc = assoc(&[kde, general], &[], &[]);
    assert_eq!(
        assoc.default_for(&mt("text/plain"), &db()),
        Some(id("kate.desktop"))
    );
    assert_eq!(
        assoc.apps_for(&mt("text/plain"), &db()),
        ids(&["kate.desktop", "gedit.desktop"])
    );
}

#[test]
fn parent_types_supply_apps_after_the_types_own() {
    let file = "[Default Applications]\ntext/plain=editor.desktop\n\
                application/x-ruby=ruby-ide.desktop\napplication/octet-stream=hex.desktop\n";
    let cache = "[MIME Cache]\ntext/x-python=py.desktop;editor.desktop\n";
    let assoc = assoc(&[file], &[cache], &[]);
    let db = db();
    let rows = [
        ("application/x-ruby", Some("ruby-ide.desktop")),
        ("application/x-shellscript", Some("editor.desktop")),
        ("text/x-python", Some("py.desktop")),
        ("text/x-unlisted", Some("editor.desktop")),
        ("image/png", Some("hex.desktop")),
    ];
    for (kind, want) in rows {
        assert_eq!(assoc.default_for(&mt(kind), &db), want.map(id), "{kind}");
    }
    assert_eq!(
        assoc.apps_for(&mt("application/x-ruby"), &db),
        ids(&["ruby-ide.desktop", "editor.desktop", "hex.desktop"])
    );
    assert_eq!(
        assoc.apps_for(&mt("text/x-python"), &db),
        ids(&["py.desktop", "editor.desktop", "hex.desktop"])
    );
}

#[test]
fn no_default_is_none_and_the_file_manager_is_the_folder_default() {
    let file = "[Default Applications]\ninode/directory=files.desktop\n";
    let with_files = assoc(&[file], &[], &[]);
    assert_eq!(with_files.file_manager_default(), Some(id("files.desktop")));
    assert_eq!(with_files.default_for(&mt("video/mp4"), &db()), None);
    assert_eq!(assoc(&[], &[], &[]).file_manager_default(), None);
}
