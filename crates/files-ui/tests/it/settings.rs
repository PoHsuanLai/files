//! files' settings file: the defaults, a bad value, the schema, what each choice does, and the
//! window following the file. Every test reads and writes a scratch config folder.

use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use ds_settings::ConfigRoot;
use ds_settings::schema::Schema;
use files_core::{Behaviour, FileName, FolderPath, ItemRef};
use files_trash::{FakeTrash, Trash};
use files_ui::settings::{self, FilesDoc, NewWindow, Switch};
use files_ui::{PlaceKind, Preferences, Sidebar};
use tempfile::TempDir;
use tokio::runtime::Handle;

/// A scratch config folder whose `files/settings.toml` says `text`.
fn written(text: &str) -> (TempDir, ConfigRoot) {
    let dir = tempfile::tempdir().unwrap();
    std::fs::create_dir_all(dir.path().join("files")).unwrap();
    std::fs::write(dir.path().join("files").join("settings.toml"), text).unwrap();
    let root = ConfigRoot::Scratch(dir.path().to_path_buf());
    (dir, root)
}

fn behaving(behaviour: Behaviour) -> Preferences {
    Preferences {
        behaviour,
        ..Preferences::default()
    }
}

#[test]
fn with_no_file_every_choice_is_finders() {
    let dir = tempfile::tempdir().unwrap();
    let doc = settings::load(&ConfigRoot::Scratch(dir.path().to_path_buf()));
    assert_eq!(doc, FilesDoc::default());
    assert_eq!(doc.new_window(), NewWindow::Home);
    let finders = Behaviour {
        folders_first: false,
        ask_before_emptying_trash: true,
        ask_before_changing_extension: true,
    };
    assert_eq!(doc.preferences(), behaving(finders));
    assert_eq!(doc.preferences().sidebar, Sidebar::default());
    assert!(toml::to_string(&doc).unwrap().contains("version = 1"));
}

#[test]
fn each_choice_changes_what_the_window_does() {
    let ask = |ask_before_emptying_trash, ask_before_changing_extension| Behaviour {
        ask_before_emptying_trash,
        ask_before_changing_extension,
        ..Behaviour::default()
    };
    let hidden = Sidebar::default()
        .leaving_out(PlaceKind::Home)
        .leaving_out(PlaceKind::Videos)
        .leaving_out(PlaceKind::Computer)
        .with_disks(false);
    let table = [
        (
            "folders on top",
            "[advanced]\nfolders_on_top = \"on\"\n",
            behaving(Behaviour {
                folders_first: true,
                ..Behaviour::default()
            }),
        ),
        (
            "no warning about an extension",
            "[advanced]\nwarn_changing_extension = \"off\"\n",
            behaving(ask(true, false)),
        ),
        (
            "no warning about the Trash",
            "[advanced]\nwarn_emptying_trash = \"off\"\n",
            behaving(ask(false, true)),
        ),
        (
            "rows left out of the sidebar",
            "[sidebar]\nhome = \"hide\"\nvideos = \"hide\"\ncomputer = \"hide\"\ndisks = \"hide\"\n",
            Preferences {
                sidebar: hidden,
                ..Preferences::default()
            },
        ),
        (
            "rows said to show",
            "[sidebar]\nhome = \"show\"\n",
            Preferences::default(),
        ),
    ];
    for (label, text, want) in table {
        let (_dir, root) = written(text);
        assert_eq!(settings::load(&root).preferences(), want, "{label}");
    }
}

#[test]
fn a_bad_value_costs_only_its_own_key() {
    let text = "[general]\nnew_windows_show = \"nowhere\"\n\
                [sidebar]\nhome = \"sideways\"\ndesktop = \"hide\"\n\
                [advanced]\nfolders_on_top = \"on\"\n";
    let (_dir, root) = written(text);
    let doc = settings::load(&root);
    assert_eq!(doc.new_window(), NewWindow::Home);
    let preferences = doc.preferences();
    assert!(preferences.behaviour.folders_first);
    assert_eq!(
        preferences.sidebar,
        Sidebar::default().leaving_out(PlaceKind::Desktop)
    );
}

#[test]
fn a_new_window_opens_where_the_person_chose() {
    let chosen = |path: &str| NewWindow::Folder(PathBuf::from(path));
    let table = [
        ("by default", "", NewWindow::Home),
        (
            "desktop",
            "new_windows_show = \"desktop\"",
            NewWindow::Desktop,
        ),
        (
            "documents",
            "new_windows_show = \"documents\"",
            NewWindow::Documents,
        ),
        (
            "downloads",
            "new_windows_show = \"downloads\"",
            NewWindow::Downloads,
        ),
        (
            "a folder",
            "new_windows_show = \"folder\"\nnew_windows_folder = \"/srv/files\"",
            chosen("/srv/files"),
        ),
        (
            "a folder, spaces around it",
            "new_windows_show = \"folder\"\nnew_windows_folder = \"  ~/Work \"",
            chosen("~/Work"),
        ),
        (
            "a folder with none written",
            "new_windows_show = \"folder\"",
            NewWindow::Home,
        ),
        (
            "a path with no folder chosen",
            "new_windows_folder = \"/srv/files\"",
            NewWindow::Home,
        ),
    ];
    for (label, general, want) in table {
        let (_dir, root) = written(&format!("[general]\n{general}\n"));
        assert_eq!(settings::load(&root).new_window(), want, "{label}");
    }
}

const KEYS: [&str; 15] = [
    "general.new_windows_show",
    "general.new_windows_folder",
    "sidebar.home",
    "sidebar.desktop",
    "sidebar.documents",
    "sidebar.downloads",
    "sidebar.pictures",
    "sidebar.music",
    "sidebar.videos",
    "sidebar.computer",
    "sidebar.disks",
    "advanced.folders_on_top",
    "advanced.warn_changing_extension",
    "advanced.warn_emptying_trash",
    "advanced.remove_old_trash",
];

#[test]
fn the_schema_names_every_key_of_files_own_file() {
    let schema = settings::schema();
    assert_eq!(schema.app.0, "files");
    // The Settings app skips a schema whose file is not `<app>/<name>.toml`.
    assert_eq!(schema.file.0, "files/settings.toml");
    let paths: Vec<&str> = schema.key.iter().map(|key| key.path.0.as_str()).collect();
    assert_eq!(paths, KEYS);
}

#[test]
fn every_default_in_the_schema_is_what_the_file_would_say() {
    let stored = toml::Value::try_from(FilesDoc::default()).unwrap();
    for key in settings::schema().key {
        let found = key
            .path
            .0
            .split('.')
            .try_fold(&stored, |at, part| at.get(part));
        assert_eq!(found, Some(&key.default), "{}", key.path.0);
    }
}

#[test]
fn the_written_schema_reads_back_and_words_the_folder_choice() {
    let dir = tempfile::tempdir().unwrap();
    let written = settings::write_schema(dir.path()).unwrap();
    assert_eq!(written, dir.path().join("files.settings.toml"));
    let read = Schema::from_toml(&std::fs::read_to_string(&written).unwrap()).unwrap();
    assert_eq!(read, settings::schema());
    let show = read
        .key
        .iter()
        .find(|key| key.path.0 == "general.new_windows_show")
        .unwrap();
    assert_eq!(show.labels.of("folder"), Some("A folder I choose"));
}

#[tokio::test]
async fn a_change_of_the_file_reaches_the_window() {
    let dir = tempfile::tempdir().unwrap();
    let root = ConfigRoot::Scratch(dir.path().to_path_buf());
    let trash = Arc::new(FakeTrash::new(dir.path().join("trash")));
    let mut feed = settings::follow(root.clone(), &Handle::current(), trash);
    assert_eq!(*feed.borrow(), Preferences::default());

    let mut doc = FilesDoc::default();
    doc.advanced.folders_on_top = Switch::On;
    settings::store(root).save(&doc).unwrap();
    tokio::time::timeout(Duration::from_secs(10), feed.changed())
        .await
        .unwrap()
        .unwrap();
    assert!(feed.borrow().behaviour.folders_first);
}

#[tokio::test]
async fn old_items_leave_the_trash_only_when_the_person_asked() {
    let table = [("asked", Switch::On, 0), ("not asked", Switch::Off, 1)];
    for (label, switch, left) in table {
        let dir = tempfile::tempdir().unwrap();
        let root = ConfigRoot::Scratch(dir.path().to_path_buf());
        let home = dir.path().join("home");
        std::fs::create_dir(&home).unwrap();
        std::fs::write(home.join("old.txt"), "x").unwrap();
        let trash = Arc::new(FakeTrash::new(dir.path().join("trash")));
        // The fake dates what it holds one second after 1970: long past thirty days.
        let item = ItemRef::new(
            FolderPath::new(&home).unwrap(),
            FileName::new("old.txt").unwrap(),
        );
        trash.trash(&[item]).unwrap();
        let mut doc = FilesDoc::default();
        doc.advanced.remove_old_trash = switch;
        settings::store(root.clone()).save(&doc).unwrap();

        let _feed = settings::follow(root, &Handle::current(), trash.clone());
        if switch == Switch::Off {
            // Give a sweep that should not happen the time to show itself.
            tokio::time::sleep(Duration::from_millis(300)).await;
        }
        for _ in 0..100 {
            if trash.list().unwrap().len() == left {
                break;
            }
            tokio::time::sleep(Duration::from_millis(50)).await;
        }
        assert_eq!(trash.list().unwrap().len(), left, "{label}");
    }
}
