//! The words of the context menu and the toolbar.

use files_core::ViewMode;

use crate::menu::Command;

/// The words of the line that opens the sheet of every installed app.
pub const OTHER_APP: &str = "Other\u{2026}";

/// The menu item's words for `command`.
pub fn command_title(command: Command) -> &'static str {
    match command {
        Command::Open => "Open",
        Command::OpenInNewTab => "Open in New Tab",
        Command::OpenWith => "Open With",
        Command::QuickLook => "Quick Look",
        Command::GetInfo | Command::GetInfoFolder => "Get Info",
        Command::CopyAsPathname => "Copy as Pathname",
        Command::MoveToTrash => "Move to Trash",
        Command::Rename => "Rename",
        Command::Copy => "Copy",
        Command::Paste => "Paste",
        Command::NewFolder => "New Folder",
        Command::PutBack => "Put Back",
        Command::DeleteImmediately => "Delete Immediately\u{2026}",
        Command::EmptyTrash => "Empty Trash\u{2026}",
        Command::DownloadNow => "Download Now",
        Command::RemoveDownload => "Remove Download",
        Command::KeepDownloaded => "Keep Downloaded",
        Command::SaveCopy => "Save a Copy",
        Command::ShowPreview => "Show Preview",
        Command::HidePreview => "Hide Preview",
    }
}

/// The Open With line of the default app called `name`.
pub fn default_app_title(name: &str) -> String {
    format!("{name} (default)")
}

/// The view switcher's words for `mode`.
pub fn view_title(mode: ViewMode) -> &'static str {
    match mode {
        ViewMode::Icons => "Icons",
        // The list, and a mode files-core adds later until the switcher names it.
        ViewMode::List | _ => "List",
    }
}
