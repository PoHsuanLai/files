//! The session bus: one running Files, and the file manager service it offers.

use std::sync::Arc;

use files_core::{FolderPath, Location};
use files_desktop::{
    Activate, ActivationToken, BusActivate, Claim, InstanceError, Served, Show, ShowRequest, Sink,
    StartupId, claim, forward, session,
};
use files_ui::{Requests, channel};
use tokio::runtime::Handle;

use crate::args::Started;
use crate::bridge::request_of;
use crate::launcher::NoBus;

/// This launch is the app: what it asks of its window and how it starts apps over the bus.
pub struct Running {
    /// The asks that arrive on the bus.
    pub requests: Requests,
    /// How apps that want the bus are started.
    pub activate: Arc<dyn Activate>,
    /// The names, kept for as long as the window runs.
    _served: Option<Served>,
}

/// What the claim decided.
pub enum Role {
    /// This launch is the app.
    App(Running),
    /// Another Files has the names and was told; this launch is done.
    Handed,
}

/// The token the desktop gave this launch, if it did.
pub fn launch_token() -> Option<ActivationToken> {
    ["XDG_ACTIVATION_TOKEN", "DESKTOP_STARTUP_ID"]
        .into_iter()
        .find_map(|name| std::env::var(name).ok())
        .and_then(ActivationToken::new)
}

/// Becomes the running Files, or hands `folder` to the one that is. When the bus cannot be used
/// the launch is a plain window with no services, and says so on stderr.
pub async fn join(
    started: Started,
    folder: FolderPath,
    token: Option<ActivationToken>,
    runtime: Handle,
) -> Role {
    let (sender, requests) = channel();
    let offline = |requests: Requests, said: InstanceError| {
        eprintln!("files: {said}");
        Role::App(Running {
            requests,
            activate: Arc::new(NoBus),
            _served: None,
        })
    };
    let connection = match session().await {
        Ok(connection) => connection,
        Err(said) => return offline(requests, said),
    };
    let sink: Sink = Arc::new(move |show: Show| {
        if let Some(request) = request_of(show) {
            sender.send(request);
        }
    });
    let activate = Arc::new(BusActivate::new(connection.clone(), runtime));
    match claim(&connection, sink).await {
        Ok(Claim::First(served)) => Role::App(Running {
            requests,
            activate,
            _served: Some(served),
        }),
        Ok(Claim::Running) => match hand_over(&connection, started, folder, token).await {
            Ok(()) => Role::Handed,
            Err(said) => {
                eprintln!("files: {said}");
                Role::App(Running {
                    requests,
                    activate,
                    _served: None,
                })
            }
        },
        Err(said) => offline(requests, said),
    }
}

/// Tells the running Files to show `folder`, with `token` to raise itself. A launch the bus made
/// carries no request of its own: the call that started it follows.
async fn hand_over(
    connection: &files_desktop::Bus,
    started: Started,
    folder: FolderPath,
    token: Option<ActivationToken>,
) -> Result<(), InstanceError> {
    match started {
        Started::ByBus => Ok(()),
        Started::ByPerson => {
            let show = Show {
                request: ShowRequest::Folders(vec![Location::Folder(folder)]),
                startup: token.and_then(|token| StartupId::new(token.as_str())),
            };
            forward(connection, &show).await
        }
    }
}
