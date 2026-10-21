//! Opening files in an app that is started over the bus (`DBusActivatable=true`).

use std::collections::HashMap;
use std::sync::mpsc;
use std::time::Duration;

use tokio::runtime::Handle;
use zbus::Connection;
use zbus::zvariant::Value;

use super::spawn::StartError;
use crate::token::ActivationToken;

/// How long the app has to answer.
const ANSWER_WITHIN: Duration = Duration::from_secs(10);

/// Asks an app, by its bus name, to open addresses.
pub trait Activate: Send + Sync {
    /// Calls `org.freedesktop.Application.Open` on `bus_name` with `uris`.
    fn open(
        &self,
        bus_name: &str,
        uris: &[String],
        token: Option<&ActivationToken>,
    ) -> Result<(), StartError>;
}

/// The object an activatable app serves: its bus name as a path (`org.a.B` is `/org/a/B`, and a
/// `-` is a `_`).
fn object_path(bus_name: &str) -> String {
    format!("/{}", bus_name.replace('.', "/").replace('-', "_"))
}

/// Activates over the session bus.
///
/// `open` waits for the app's answer, so call it from a thread that is not one of the runtime's
/// workers (the window's thread, or `spawn_blocking`).
#[derive(Debug, Clone)]
pub struct BusActivate {
    connection: Connection,
    runtime: Handle,
}

impl BusActivate {
    /// Activation over `connection`, with calls driven on `runtime`.
    pub fn new(connection: Connection, runtime: Handle) -> BusActivate {
        BusActivate {
            connection,
            runtime,
        }
    }
}

impl Activate for BusActivate {
    fn open(
        &self,
        bus_name: &str,
        uris: &[String],
        token: Option<&ActivationToken>,
    ) -> Result<(), StartError> {
        let connection = self.connection.clone();
        let name = bus_name.to_owned();
        let uris = uris.to_vec();
        let mut platform: HashMap<String, Value<'static>> = HashMap::new();
        if let Some(token) = token {
            platform.insert(
                "activation-token".to_owned(),
                Value::from(token.as_str().to_owned()),
            );
            platform.insert(
                "desktop-startup-id".to_owned(),
                Value::from(token.as_str().to_owned()),
            );
        }
        let (sender, answer) = mpsc::channel();
        drop(self.runtime.spawn(async move {
            let path = object_path(&name);
            let reply = connection
                .call_method(
                    Some(name.as_str()),
                    path.as_str(),
                    Some("org.freedesktop.Application"),
                    "Open",
                    &(uris, platform),
                )
                .await;
            let _ = sender.send(reply.map(drop).map_err(|e| StartError::new(e.to_string())));
        }));
        answer
            .recv_timeout(ANSWER_WITHIN)
            .unwrap_or_else(|_| Err(StartError::new("It did not answer.")))
    }
}

/// Records what it was asked to open, for tests.
#[cfg(feature = "testing")]
#[derive(Debug, Default)]
pub struct FakeActivate {
    calls: std::sync::Mutex<Vec<ActivateCall>>,
}

/// One call a [`FakeActivate`] heard.
#[cfg(feature = "testing")]
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ActivateCall {
    /// The app's bus name.
    pub bus_name: String,
    /// The addresses to open.
    pub uris: Vec<String>,
    /// The activation token it was given.
    pub token: Option<ActivationToken>,
}

#[cfg(feature = "testing")]
impl FakeActivate {
    /// A fake with no calls yet.
    pub fn new() -> FakeActivate {
        FakeActivate::default()
    }

    /// The calls heard, in order.
    pub fn calls(&self) -> Vec<ActivateCall> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(feature = "testing")]
impl Activate for FakeActivate {
    fn open(
        &self,
        bus_name: &str,
        uris: &[String],
        token: Option<&ActivationToken>,
    ) -> Result<(), StartError> {
        self.calls
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(ActivateCall {
                bus_name: bus_name.to_owned(),
                uris: uris.to_vec(),
                token: token.cloned(),
            });
        Ok(())
    }
}
