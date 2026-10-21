//! Putting the provider on a connection.

use std::sync::Arc;

use docket_client::serve_on_with;
use docket_core::{Manifest, ValidManifest, validate};
use docket_dbus::callers::{Identity, intentd_only};
use porter_core::AppName;
use zbus::Connection;

use super::ask::Window;
use super::names::{APP_BUS_NAME, MANIFEST};
use super::provider::{Files, NoPrompt, Snapshot};
use crate::error::IntentError;

/// The shipped file, validated the way the router validates it.
pub(super) fn load_manifest() -> Result<ValidManifest, IntentError> {
    let parsed: Manifest =
        toml::from_str(MANIFEST).map_err(|error| IntentError::Manifest(error.to_string()))?;
    validate(parsed).map_err(|error| IntentError::Manifest(error.to_string()))
}

/// Offers Files' actions on `connection`, answered by `window`. The connection keeps serving for
/// as long as it lives. Only intentd is answered, known by its unit (`intentd.service`) as
/// `/proc` reports it. `connection` is the one that owns `org.quire.Files`.
pub async fn offer<W: Window>(connection: &Connection, window: Arc<W>) -> Result<(), IntentError> {
    let callers = Identity::on_system(connection.clone(), intentd_only());
    serve(connection, window, callers).await
}

/// [`offer`] on a test's private bus, where every connection is one process and no unit names
/// intentd: `router` stands in for it.
#[cfg(feature = "testing")]
pub async fn offer_for_router<W: Window>(
    connection: &Connection,
    window: Arc<W>,
    router: &Connection,
) -> Result<(), IntentError> {
    use docket_dbus::callers::Role;
    let callers = Identity::on_system(connection.clone(), intentd_only());
    callers.introduce_app(router, "org.quire.Intents", Role::Agent);
    serve(connection, window, callers).await
}

async fn serve<W: Window>(
    connection: &Connection,
    window: Arc<W>,
    callers: Identity,
) -> Result<(), IntentError> {
    let manifest = load_manifest()?;
    let app = AppName::parse(APP_BUS_NAME)
        .map_err(|_| IntentError::Manifest("bad app name".to_owned()))?;
    let provider = Files {
        manifest,
        app: app.clone(),
        window,
    };
    serve_on_with(
        connection,
        provider,
        Snapshot { app },
        NoPrompt,
        Arc::new(callers),
    )
    .await
    .map_err(|error| IntentError::Offer(error.to_string()))
}
