//! The token that lets a program raise its window.

/// An activation token, as the desktop hands one to a program that was started by a click
/// (`XDG_ACTIVATION_TOKEN`, `DESKTOP_STARTUP_ID`, the `startup_id` of a bus call).
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct ActivationToken(String);

impl ActivationToken {
    /// The token `text`; an empty text is no token, so `None`.
    pub fn new(text: impl Into<String>) -> Option<ActivationToken> {
        let text = text.into();
        (!text.is_empty()).then_some(ActivationToken(text))
    }

    /// The token as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
