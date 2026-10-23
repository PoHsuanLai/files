//! The address of an online document.

/// Why a text is not an address Files opens.
#[derive(Debug, Clone, Copy, PartialEq, Eq, thiserror::Error)]
#[non_exhaustive]
pub enum UrlError {
    /// Not a web address (`https:` or `http:`).
    #[error("That isn\u{2019}t a web address.")]
    NotWeb,
    /// Spaces or control characters.
    #[error("That web address has stray characters.")]
    Stray,
}

/// The web address of a document that lives only online (a Google Doc): opened in the browser.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct OnlineUrl(String);

impl OnlineUrl {
    /// `text`, when it is an `https:` or `http:` address with nothing stray in it.
    pub fn new(text: impl Into<String>) -> Result<OnlineUrl, UrlError> {
        let text = text.into();
        let web = text.starts_with("https://") || text.starts_with("http://");
        if !web {
            return Err(UrlError::NotWeb);
        }
        if text.chars().any(|c| c.is_control() || c.is_whitespace()) {
            return Err(UrlError::Stray);
        }
        Ok(OnlineUrl(text))
    }

    /// The address as text.
    pub fn as_str(&self) -> &str {
        &self.0
    }
}
