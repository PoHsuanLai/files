//! Opening an online document in the browser, through the desktop portal's `OpenURI`, which the
//! person's own settings answer (their default browser).

use std::collections::HashMap;

use files_core::OnlineUrl;
use zbus::Connection;
use zbus::zvariant::{OwnedObjectPath, Value};

/// Why an address was not opened.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum OpenUrlError {
    /// The desktop would not take the request.
    #[error("The document couldn\u{2019}t be opened in the browser: {0}")]
    Portal(#[from] zbus::Error),
}

#[zbus::proxy(
    interface = "org.freedesktop.portal.OpenURI",
    default_service = "org.freedesktop.portal.Desktop",
    default_path = "/org/freedesktop/portal/desktop",
    gen_blocking = false
)]
trait OpenUri {
    /// Asks the desktop to open `uri` with the app the person chose for it.
    fn open_uri(
        &self,
        parent_window: &str,
        uri: &str,
        options: HashMap<&str, Value<'_>>,
    ) -> zbus::Result<OwnedObjectPath>;
}

/// Opens `url` in the person's browser. `parent_window` is empty: the window has no handle the
/// portal could use.
pub async fn open_url(connection: &Connection, url: &OnlineUrl) -> Result<(), OpenUrlError> {
    let portal = OpenUriProxy::new(connection).await?;
    portal
        .open_uri("", url.as_str(), HashMap::new())
        .await
        .map(|_| ())
        .map_err(OpenUrlError::from)
}
