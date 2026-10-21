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
use files_model::Input;

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
    groups
        .iter()
        .flatten()
        .fold(Registration::new(app), |registration, (action, chord)| {
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
    // A space starts nothing: it only continues a name already begun.
    if text == " " && browse.state.read().typed().is_empty() {
        return ActionTaken::No;
    }
    for c in text.chars() {
        browse.send(Input::Type(c));
    }
    ActionTaken::Yes
}
