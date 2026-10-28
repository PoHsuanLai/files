//! What Get Info, the status line and the drive's empty state say about cloud drives. Pure.

use files_core::{CloudDrive, CloudItem, CloudKind, CloudState, DriveState, Usage};

use super::{InfoRow, size_words};

/// A drive's state as Get Info says it.
pub fn drive_state_words(state: DriveState) -> &'static str {
    match state {
        DriveState::Starting => "Starting up",
        DriveState::Ready => "Up to date",
        DriveState::Syncing => "Syncing",
        DriveState::Offline => "Offline",
        DriveState::Paused => "Syncing paused",
        DriveState::NeedsSignIn => "Needs sign-in",
        DriveState::Error | _ => "Something went wrong",
    }
}

fn percent(item: &CloudItem) -> String {
    match item.share {
        Some(share) => format!(", {}%", share.permille() / 10),
        None => String::new(),
    }
}

/// Where an item stands, as Get Info says it.
pub fn item_state_words(item: &CloudItem) -> String {
    if item.kind == CloudKind::Online {
        return "Opens in your browser".to_owned();
    }
    match item.state {
        CloudState::Cloud => "In the cloud".to_owned(),
        CloudState::Downloading => format!("Downloading{}", percent(item)),
        CloudState::Downloaded => "Downloaded".to_owned(),
        CloudState::Uploading => format!("Uploading{}", percent(item)),
        CloudState::Waiting => "Waiting to upload".to_owned(),
        CloudState::Error => item.words.as_ref().map_or_else(
            || "Couldn\u{2019}t sync".to_owned(),
            |words| words.to_string(),
        ),
        CloudState::Conflict => "Two versions of this file are kept".to_owned(),
        _ => "In the cloud".to_owned(),
    }
}

/// `used` of `total`, or just what is used.
fn room_words(usage: &Usage) -> Option<String> {
    match (usage.used, usage.total) {
        (Some(used), Some(total)) => Some(format!(
            "{} of {} used",
            size_words(used),
            size_words(total)
        )),
        (Some(used), None) => Some(format!("{} used", size_words(used))),
        (None, Some(total)) => Some(format!("{} available in all", size_words(total))),
        (None, None) => None,
    }
}

/// What the status line says while a drive's folder is shown: the count, what is on this
/// computer, and the room when the provider says.
pub fn status_words(drive: &CloudDrive, items: usize) -> String {
    let noun = if items == 1 { "item" } else { "items" };
    let mut parts = vec![
        format!("{items} {noun}"),
        format!("{} on this computer", size_words(drive.usage.local)),
    ];
    parts.extend(room_words(&drive.usage));
    parts.join(", ")
}

/// The rows Get Info adds for the drive's own folder.
pub fn drive_rows(drive: &CloudDrive) -> Vec<InfoRow> {
    let mut rows = vec![InfoRow::new(
        "Status",
        drive_state_words(drive.state).to_owned(),
    )];
    if let Some(words) = &drive.words {
        rows.push(InfoRow::new("Message", words.to_string()));
    }
    if !drive.label.as_str().is_empty() {
        rows.push(InfoRow::new("Account", drive.label.to_string()));
    }
    rows.push(InfoRow::new(
        "On this computer",
        size_words(drive.usage.local),
    ));
    rows.extend(room_words(&drive.usage).map(|words| InfoRow::new("Storage", words)));
    rows
}

/// The rows Get Info adds for an item of a drive.
pub fn item_rows(item: &CloudItem) -> Vec<InfoRow> {
    let mut rows = vec![InfoRow::new("Status", item_state_words(item))];
    if item.kept && item.kind != CloudKind::Online {
        rows.push(InfoRow::new("Keep Downloaded", "Yes".to_owned()));
    }
    rows
}

/// The title of a drive that cannot show its files, and what the person can do about it.
pub fn trouble_title(drive: &CloudDrive) -> String {
    match (&drive.words, drive.state) {
        (Some(words), _) => words.to_string(),
        (None, DriveState::NeedsSignIn) => "Sign in to see these files.".to_owned(),
        (None, _) => "These files can\u{2019}t be shown right now.".to_owned(),
    }
}

/// The button of a drive that needs signing in.
pub const SIGN_IN_BUTTON: &str = "Open Accounts\u{2026}";
