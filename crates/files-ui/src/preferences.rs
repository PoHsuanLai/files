//! The person's choices the window follows while it is open.

use files_core::Behaviour;

use crate::places::Sidebar;

/// What the window takes from the person's settings: how it behaves, and which rows its sidebar
/// lists. The binary reads them from the settings file and sends each change through the channel
/// in [`Launch`](crate::Launch); the window applies it at once.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct Preferences {
    /// How the window behaves where the person has a choice.
    pub behaviour: Behaviour,
    /// Which rows the sidebar lists.
    pub sidebar: Sidebar,
}
