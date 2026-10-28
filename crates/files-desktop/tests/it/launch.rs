//! Opening files: which app, how many times, with what arguments and environment.

use std::path::{Path, PathBuf};
use std::sync::Arc;

use ds_helpers::mime::{Associations, DesktopId, MimeApps, MimeDb, MimeLayer};
use files_desktop::{
    ActivateCall, ActivationToken, AppChoice, AppEntry, Catalog, DesktopLauncher, EntryError,
    FakeActivate, FakeIcons, FakeSpawn, LaunchError, Launcher, installed_check,
};

const GLOBS: &str = "50:text/plain:*.txt\n50:image/png:*.png\n50:application/pdf:*.pdf\n\
                     50:text/html:*.html\n50:application/x-a:*.amb\n50:application/x-b:*.amb\n";

const MIMEAPPS: &str = "[Default Applications]\ntext/plain=editor.desktop\n\
                        image/png=viewer.desktop\napplication/pdf=viewer.desktop\n\
                        text/html=terminal.desktop\n";

const APPS: [(&str, &str); 5] = [
    (
        "editor.desktop",
        "[Desktop Entry]\nType=Application\nName=Editor\nExec=editor %F\n",
    ),
    (
        "viewer.desktop",
        "[Desktop Entry]\nType=Application\nName=Viewer\nIcon=viewer-icon\n\
         Exec=viewer --icon-name %c %f\n",
    ),
    (
        "terminal.desktop",
        "[Desktop Entry]\nType=Application\nName=Vim\nTerminal=true\nExec=vim %F\n",
    ),
    (
        "org.example.Bus.desktop",
        "[Desktop Entry]\nType=Application\nName=Bus App\nIcon=not-in-theme\nDBusActivatable=true\nExec=bus-app %U\n",
    ),
    (
        "quoted.desktop",
        "[Desktop Entry]\nType=Application\nName=Quoted\nIcon=/opt/My App/icon.png\nPath=/srv/work\n\
         Exec=\"/opt/My App/run\" --open %u\n",
    ),
];

struct Rig {
    launcher: DesktopLauncher,
    spawn: Arc<FakeSpawn>,
    activate: Arc<FakeActivate>,
    theme: Arc<FakeIcons>,
}

fn id(text: &str) -> DesktopId {
    DesktopId::new(text).unwrap()
}

fn rig(mimeapps: &str, spawn: FakeSpawn) -> Rig {
    let db = MimeDb::from_layers(&[MimeLayer {
        globs2: GLOBS,
        ..MimeLayer::default()
    }]);
    let apps: Vec<AppEntry> = APPS
        .iter()
        .map(|(name, text)| {
            let path = PathBuf::from(format!("/apps/{name}"));
            AppEntry::from_text(id(name), &path, text).unwrap()
        })
        .collect();
    let associations = Associations::new(
        vec![MimeApps::parse(mimeapps)],
        Vec::new(),
        installed_check(&apps),
    );
    let spawn = Arc::new(spawn);
    let activate = Arc::new(FakeActivate::new());
    let theme = Arc::new(FakeIcons::new().with("viewer-icon", "/theme/viewer-icon.png"));
    let catalog = Catalog {
        db,
        associations,
        apps,
    };
    let launcher = DesktopLauncher::builder(catalog, activate.clone())
        .spawn(spawn.clone())
        .icons(theme.clone())
        .build();
    Rig {
        launcher,
        spawn,
        activate,
        theme,
    }
}

fn targets(paths: &[&str]) -> Vec<PathBuf> {
    paths.iter().map(PathBuf::from).collect()
}

fn argvs(rig: &Rig) -> Vec<Vec<String>> {
    rig.spawn
        .spawned()
        .iter()
        .map(|c| {
            c.argv
                .iter()
                .map(|a| a.to_string_lossy().into_owned())
                .collect()
        })
        .collect()
}

fn strs(items: &[&str]) -> Vec<String> {
    items.iter().map(|s| (*s).to_owned()).collect()
}

#[test]
fn each_default_app_starts_once_for_its_files_or_once_per_file() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let files = targets(&["/p/a.txt", "/p/c.png", "/p/b.txt", "/p/d.pdf"]);
    rig.launcher.open(&files, AppChoice::Default, None).unwrap();
    assert_eq!(
        argvs(&rig),
        vec![
            strs(&["editor", "/p/a.txt", "/p/b.txt"]),
            strs(&["viewer", "--icon-name", "Viewer", "/p/c.png"]),
            strs(&["viewer", "--icon-name", "Viewer", "/p/d.pdf"]),
        ]
    );
}

#[test]
fn a_chosen_app_opens_everything() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let files = targets(&["/p/a.txt", "/p/b.txt"]);
    let with = AppChoice::App(id("viewer.desktop"));
    rig.launcher.open(&files, with, None).unwrap();
    assert_eq!(
        argvs(&rig),
        vec![
            strs(&["viewer", "--icon-name", "Viewer", "/p/a.txt"]),
            strs(&["viewer", "--icon-name", "Viewer", "/p/b.txt"]),
        ]
    );
}

#[test]
fn the_token_reaches_the_started_program() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let token = ActivationToken::new("tok-9");
    rig.launcher
        .open(&targets(&["/p/a.txt"]), AppChoice::Default, token)
        .unwrap();
    rig.launcher
        .open(&targets(&["/p/b.txt"]), AppChoice::Default, None)
        .unwrap();
    let spawned = rig.spawn.spawned();
    let pair = |name: &str| (name.to_owned(), "tok-9".to_owned());
    assert_eq!(
        spawned[0].env,
        vec![pair("XDG_ACTIVATION_TOKEN"), pair("DESKTOP_STARTUP_ID")]
    );
    assert!(spawned[1].env.is_empty());
}

#[test]
fn quoted_commands_addresses_and_the_working_folder() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let with = AppChoice::App(id("quoted.desktop"));
    rig.launcher
        .open(&targets(&["/p/a b.txt"]), with, None)
        .unwrap();
    assert_eq!(
        argvs(&rig),
        vec![strs(&["/opt/My App/run", "--open", "file:///p/a%20b.txt"])]
    );
    assert_eq!(
        rig.spawn.spawned()[0].work_dir,
        Some(PathBuf::from("/srv/work"))
    );
}

#[test]
fn a_bus_app_is_asked_over_the_bus_with_addresses_and_the_token() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let with = AppChoice::App(id("org.example.Bus.desktop"));
    let token = ActivationToken::new("tok-3");
    rig.launcher
        .open(&targets(&["/p/a b.txt", "/p/c.txt"]), with, token.clone())
        .unwrap();
    assert!(rig.spawn.spawned().is_empty());
    assert_eq!(
        rig.activate.calls(),
        vec![ActivateCall {
            bus_name: "org.example.Bus".to_owned(),
            uris: strs(&["file:///p/a%20b.txt", "file:///p/c.txt"]),
            token,
        }]
    );
}

#[test]
fn what_cannot_be_opened_says_so_in_plain_words() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let open = |paths: &[&str], with| rig.launcher.open(&targets(paths), with, None);

    // Nothing opens it; the first group still ran.
    let error = open(&["/p/a.txt", "/p/x.zzz"], AppChoice::Default).unwrap_err();
    assert_eq!(error, LaunchError::NoApp("x.zzz".to_owned()));
    assert_eq!(
        error.to_string(),
        "There is no app set to open \u{201c}x.zzz\u{201d}."
    );
    assert_eq!(argvs(&rig), vec![strs(&["editor", "/p/a.txt"])]);

    // Two types fit a name equally well: no guess, so the unknown-type default, which is none.
    let error = open(&["/p/y.amb"], AppChoice::Default).unwrap_err();
    assert_eq!(error, LaunchError::NoApp("y.amb".to_owned()));

    let missing = AppChoice::App(id("gone.desktop"));
    let error = open(&["/p/a.txt"], missing).unwrap_err();
    assert_eq!(error, LaunchError::NoApp("a.txt".to_owned()));

    let error = open(&["/p/page.html"], AppChoice::Default).unwrap_err();
    assert_eq!(error, LaunchError::NeedsTerminal("Vim".to_owned()));
    assert_eq!(argvs(&rig).len(), 1);
}

#[test]
fn a_name_the_database_cannot_place_goes_to_the_unknown_type_default() {
    let mimeapps = format!("{MIMEAPPS}application/octet-stream=editor.desktop\n");
    let rig = rig(&mimeapps, FakeSpawn::new());
    let files = targets(&["/p/x.zzz", "/p/y.amb"]);
    rig.launcher.open(&files, AppChoice::Default, None).unwrap();
    assert_eq!(argvs(&rig), vec![strs(&["editor", "/p/x.zzz", "/p/y.amb"])]);
}

#[test]
fn a_program_that_will_not_start_is_reported_with_the_app_and_why() {
    let rig = rig(MIMEAPPS, FakeSpawn::refusing("No such file or directory"));
    let error = rig
        .launcher
        .open(&targets(&["/p/a.txt"]), AppChoice::Default, None)
        .unwrap_err();
    assert_eq!(
        error,
        LaunchError::Failed {
            app: "Editor".to_owned(),
            reason: "No such file or directory".to_owned(),
        }
    );
}

#[test]
fn entries_that_are_not_apps_are_refused() {
    let path = Path::new("/apps/x.desktop");
    let rows = [
        ("[Desktop Entry]\nType=Link\nName=L\nURL=x\n", "NotAnApp"),
        (
            "[Desktop Entry]\nType=Application\nName=NoExec\n",
            "NoCommand",
        ),
        ("Name=before any group\n", "NoEntry"),
    ];
    for (text, want) in rows {
        let error = AppEntry::from_text(id("x.desktop"), path, text).unwrap_err();
        let kind = match error {
            EntryError::NotAnApp(_) => "NotAnApp",
            EntryError::NoCommand(_) => "NoCommand",
            EntryError::NoEntry => "NoEntry",
            EntryError::Unreadable => "Unreadable",
            _ => "other",
        };
        assert_eq!(kind, want, "{text}");
    }
}

fn offers(rig: &Rig, paths: &[&str]) -> Vec<(String, bool)> {
    rig.launcher
        .apps_for(&targets(paths))
        .into_iter()
        .map(|offer| (offer.name, offer.is_default))
        .collect()
}

/// A label, the files, and the apps offered for them in order.
type OfferCase<'a> = (&'a str, Vec<&'a str>, Vec<(&'a str, bool)>);

#[test]
fn the_apps_offered_for_files_put_the_default_first() {
    let added = format!("{MIMEAPPS}[Added Associations]\ntext/plain=viewer.desktop;\n");
    let rig = rig(&added, FakeSpawn::new());
    let cases: Vec<OfferCase> = vec![
        (
            "one type, two apps",
            vec!["/p/a.txt"],
            vec![("Editor", true), ("Viewer", false)],
        ),
        ("one default only", vec!["/p/c.png"], vec![("Viewer", true)]),
        (
            "the apps both types share",
            vec!["/p/c.png", "/p/d.pdf"],
            vec![("Viewer", true)],
        ),
        (
            "the first type leads",
            vec!["/p/a.txt", "/p/c.png"],
            vec![("Viewer", false)],
        ),
        ("no type is known", vec!["/p/noext"], vec![]),
        ("no files", vec![], vec![]),
    ];
    for (label, files, want) in cases {
        let want: Vec<(String, bool)> = want
            .into_iter()
            .map(|(name, is_default)| (name.to_owned(), is_default))
            .collect();
        assert_eq!(offers(&rig, &files), want, "{label}");
    }
}

#[test]
fn every_installed_app_is_offered_for_the_other_sheet() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let names: Vec<String> = rig
        .launcher
        .all_apps()
        .into_iter()
        .map(|offer| offer.name)
        .collect();
    assert_eq!(
        names,
        strs(&["Editor", "Viewer", "Vim", "Bus App", "Quoted"])
    );
}

#[test]
fn a_folder_is_offered_the_apps_for_folders() {
    let dir = tempfile::tempdir().unwrap();
    let rig = rig(
        "[Default Applications]\ninode/directory=editor.desktop\n",
        FakeSpawn::new(),
    );
    let shown = rig.launcher.apps_for(&[dir.path().to_path_buf()]);
    assert_eq!(shown.len(), 1);
    assert!(shown[0].is_default);
}

#[test]
fn offers_carry_the_icon_file_of_the_entry() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let icons: Vec<(String, Option<PathBuf>)> = rig
        .launcher
        .all_apps()
        .into_iter()
        .map(|offer| (offer.name, offer.icon))
        .collect();
    // (app, icon): a theme name found, an absolute path as it is, a name the theme lacks, and an
    // entry with no icon at all.
    let want = [
        ("Editor", None),
        ("Viewer", Some("/theme/viewer-icon.png")),
        ("Vim", None),
        ("Bus App", None),
        ("Quoted", Some("/opt/My App/icon.png")),
    ];
    let want: Vec<(String, Option<PathBuf>)> = want
        .into_iter()
        .map(|(name, icon)| (name.to_owned(), icon.map(PathBuf::from)))
        .collect();
    assert_eq!(icons, want);
}

#[test]
fn an_icon_is_looked_up_once_for_the_life_of_the_launcher() {
    let rig = rig(MIMEAPPS, FakeSpawn::new());
    let _ = rig.launcher.all_apps();
    let after_first = rig.theme.asked();
    // Viewer and Bus App name theme icons; Quoted's is a path and is never asked of the theme.
    assert_eq!(after_first, 2);
    let _ = rig.launcher.all_apps();
    let _ = rig.launcher.apps_for(&targets(&["/p/c.png"]));
    assert_eq!(rig.theme.asked(), after_first);
}
