//! What a key press means in the window: files' own actions, the standard ones it answers, and
//! typing a name to jump to it.
//!
//! The window never reads which modifier is held. quire's `Keys` turns a press into chordkit's
//! `Action` (the keymap follows the system's own shortcut settings), and this file declares
//! files' actions with portable chords written with `Primary` and answers the ones the window
//! offers. Standard actions are handled, never declared.

use chordkit::{Action, AppAction, AppId, Context, DefaultChord, Registration, StandardAction};
use dioxus::prelude::{
    Key, KeyboardEvent, ModifiersInteraction, ReadableExt, Signal, WritableExt, use_hook,
};
use ds::base::command::types_text;
use ds::prelude::{ActionTaken, Keys, Shown, use_keys};
use files_model::{Browser, Chooser, GoTo, InfoView, Input};

use crate::state::Browse;

/// files' name to chordkit: the first part of every action id.
const APP: &str = "files";

/// The actions files declares.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Own {
    /// Back (Command+[).
    Back,
    /// Forward (Command+]).
    Forward,
    /// The folder that holds this one (Command+Up).
    Enclosing,
    /// Open the selection (Command+Down).
    Open,
    /// Show or leave out hidden files (Command+Shift+.).
    ShowHidden,
    /// Rename the selected item in place (Return).
    Rename,
    /// Make a new folder here (Command+Shift+N).
    NewFolder,
    /// Move the selection to the Trash (Command+Backspace).
    MoveToTrash,
    /// Move what was copied here instead of copying it (Command+Option+V).
    PasteMove,
    /// Empty the Trash, after asking (Command+Shift+Backspace).
    EmptyTrash,
    /// Show the properties of the selection (Command+I).
    GetInfo,
    /// Go to a folder typed by path (Command+Shift+G).
    GoToFolder,
    /// Put the paths of the selection on the clipboard (Command+Option+C).
    CopyAsPathname,
}

/// Each action with its id and its default chords. `Primary` is Command or Ctrl. A symbol typed
/// with Shift is written both ways, as a layout may report either.
const OWN: &[(Own, &str, &[&str])] = &[
    (Own::Back, "files.back", &["Primary+["]),
    (Own::Forward, "files.forward", &["Primary+]"]),
    (Own::Enclosing, "files.enclosing", &["Primary+Up"]),
    (Own::Open, "files.open", &["Primary+Down"]),
    (
        Own::ShowHidden,
        "files.show-hidden",
        &["Primary+Shift+.", "Primary+Shift+>"],
    ),
    (Own::Rename, "files.rename", &["Return"]),
    (Own::NewFolder, "files.new-folder", &["Primary+Shift+N"]),
    (
        Own::MoveToTrash,
        "files.move-to-trash",
        &["Primary+Backspace"],
    ),
    (Own::PasteMove, "files.paste-move", &["Primary+Alt+V"]),
    (
        Own::EmptyTrash,
        "files.empty-trash",
        &["Primary+Shift+Backspace"],
    ),
    (Own::GetInfo, "files.get-info", &["Primary+I"]),
    (Own::GoToFolder, "files.go-to-folder", &["Primary+Shift+G"]),
    (
        Own::CopyAsPathname,
        "files.copy-pathname",
        &["Primary+Alt+C"],
    ),
];

/// The action an id names.
fn own_of(action: &AppAction) -> Option<Own> {
    OWN.iter()
        .find(|(_, id, _)| *id == action.id())
        .map(|(own, _, _)| *own)
}

/// The rows of one action: its id with each chord.
fn rows(id: &str, chords: &[&str]) -> Vec<(AppAction, DefaultChord)> {
    let Ok(action) = AppAction::new(id) else {
        return Vec::new();
    };
    chords
        .iter()
        .filter_map(|chord| chord.parse::<DefaultChord>().ok())
        .map(|chord| (action.clone(), chord))
        .collect()
}

/// `groups` as one registration for `app`.
fn registration(app: &AppId, groups: &[Vec<(AppAction, DefaultChord)>]) -> Registration {
    // Files has no text styles, so Command+I is Get Info rather than Italic.
    let start = Registration::new(app).forgo(StandardAction::Italic);
    groups
        .iter()
        .flatten()
        .fold(start, |registration, (action, chord)| {
            registration.action(action.clone(), *chord)
        })
}

/// Declares files' actions on `keys`. A registration is all or nothing, so each action is tried
/// on top of those already taken and one the system keeps for itself costs that action alone.
fn register(keys: &Keys) {
    let Ok(app) = AppId::new(APP) else {
        return;
    };
    let mut taken: Vec<Vec<(AppAction, DefaultChord)>> = Vec::new();
    for (_, id, chords) in OWN {
        taken.push(rows(id, chords));
        if let Err(conflict) = keys.register(&app, &registration(&app, &taken)) {
            eprintln!("files: keys: {conflict}");
            taken.pop();
        }
    }
}

/// The window's keymap, with files' actions declared in it once, at the first render.
pub(crate) fn use_registered() -> Keys {
    let keys = use_keys();
    use_hook(move || register(&keys));
    keys
}

/// Answers a key press in the window. A key the window takes is consumed; the rest goes on to
/// the focused control.
pub(crate) fn pressed(
    keys: Keys,
    browse: Browse,
    sidebar: Signal<Shown>,
    event: &KeyboardEvent,
) -> ActionTaken {
    // Read first, whatever the key: it is only meaningful inside this one event.
    let trail = browse.take_trail();
    // The name field has the keyboard while one is typed: its keys are its own.
    if browse.renaming() {
        return ActionTaken::No;
    }
    if event.key() == Key::Escape {
        return escape(browse);
    }
    // Get Info, Go to Folder and the sheet of apps are sheets over the window: the keys behind
    // them wait.
    if sheet_up(&browse.state.read()) {
        return ActionTaken::No;
    }
    match keys.action_of(event, Context::Normal) {
        Some(action) => act(&action, browse, sidebar, trail),
        None => type_ahead(browse, event),
    }
}

/// Carries out `action`, when the window answers it.
fn act(
    action: &Action,
    browse: Browse,
    mut sidebar: Signal<Shown>,
    trail: Option<files_model::Browser>,
) -> ActionTaken {
    let send = |input: Input| {
        browse.send(input);
        ActionTaken::Yes
    };
    match action {
        Action::Standard(standard) if *standard == StandardAction::SelectAll => {
            send(Input::SelectAll)
        }
        Action::Standard(standard) if *standard == StandardAction::Open => send(Input::Open),
        Action::Standard(standard) if *standard == StandardAction::Copy => send(Input::Copy),
        Action::Standard(standard) if *standard == StandardAction::Paste => send(Input::Paste),
        Action::Standard(standard) if *standard == StandardAction::Undo => send(Input::Undo),
        Action::Standard(standard) if *standard == StandardAction::QuickLook => {
            send(Input::QuickLook)
        }
        Action::Standard(standard) if *standard == StandardAction::ToggleSidebar => {
            let next = sidebar().flipped();
            sidebar.set(next);
            ActionTaken::Yes
        }
        Action::App(app) => match own_of(app) {
            Some(Own::Back) => send(Input::Back),
            Some(Own::Forward) => send(Input::Forward),
            // The table read these two as "move the cursor" first; the person meant the
            // selection they had.
            Some(Own::Enclosing) => {
                put_back(browse, trail);
                send(Input::Enclosing)
            }
            Some(Own::Open) => {
                put_back(browse, trail);
                send(Input::Open)
            }
            Some(Own::ShowHidden) => send(Input::ToggleHidden),
            Some(Own::Rename) => send(Input::BeginRename),
            Some(Own::NewFolder) => send(Input::NewFolder),
            Some(Own::MoveToTrash) => send(Input::MoveToTrash),
            Some(Own::PasteMove) => send(Input::PasteMove),
            Some(Own::EmptyTrash) => send(Input::EmptyTrash),
            Some(Own::GetInfo) => send(Input::GetInfo),
            Some(Own::GoToFolder) => send(Input::GoToFolder),
            Some(Own::CopyAsPathname) => send(Input::CopyAsPathname),
            None => ActionTaken::No,
        },
        // A standard action the window does not offer, or one chordkit adds later.
        Action::Standard(_) | _ => ActionTaken::No,
    }
}

/// Undoes the cursor move the table made for this key, if it made one.
fn put_back(browse: Browse, trail: Option<files_model::Browser>) {
    if let Some(before) = trail {
        browse.restore(before);
    }
}

/// A letter, digit or symbol typed with no command key held jumps to the name that starts with
/// what has been typed.
fn type_ahead(browse: Browse, event: &KeyboardEvent) -> ActionTaken {
    let key = event.key();
    if !types_text(&key, event.modifiers()) {
        return ActionTaken::No;
    }
    let Key::Character(text) = key else {
        return ActionTaken::No;
    };
    // A space with no name begun is Quick Look (Finder's); in the middle of one it is a letter.
    if text == " " && browse.state.read().typed().is_empty() {
        browse.send(Input::QuickLook);
        return ActionTaken::Yes;
    }
    for c in text.chars() {
        browse.send(Input::Type(c));
    }
    ActionTaken::Yes
}

/// Whether a sheet is up over the folder: Get Info, Go to Folder or the apps of Open With.
fn sheet_up(state: &Browser) -> bool {
    !matches!(state.info(), InfoView::Closed)
        || !matches!(state.go_to(), GoTo::Closed)
        || state.chooser() == Chooser::Open
}

/// Escape puts away what is up over the folder: a sheet of Go to Folder or of apps, Get Info,
/// else Quick Look. With nothing up the key goes on to whoever has it.
fn escape(browse: Browse) -> ActionTaken {
    let input = {
        let state = browse.state.read();
        if !matches!(state.go_to(), GoTo::Closed) {
            Some(Input::GoToCancel)
        } else if state.chooser() == Chooser::Open {
            Some(Input::CloseChooseApp)
        } else {
            match (state.info(), state.quick_look()) {
                (InfoView::Closed, None) => None,
                (InfoView::Closed, Some(_)) => Some(Input::CloseQuickLook),
                (InfoView::Loading(_) | InfoView::Shown(_) | InfoView::Failed(..) | _, _) => {
                    Some(Input::CloseInfo)
                }
            }
        }
    };
    match input {
        Some(input) => {
            browse.send(input);
            ActionTaken::Yes
        }
        None => ActionTaken::No,
    }
}
