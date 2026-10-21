//! One running app: the first launch owns `org.quire.Files`, a later one hands its request to it.
//!
//! The first launch also offers `org.freedesktop.FileManager1`, unless another file manager already
//! owns that name: the name is asked for without queueing and without replacing, so a person's
//! chosen file manager is never taken over. [`Served::file_manager`] says which it was, so the
//! window can offer to become the default.

pub use zbus::Connection as Bus;
use zbus::Connection;
use zbus::fdo::{DBusProxy, RequestNameFlags, RequestNameReply};
use zbus::names::BusName;

pub use crate::error::InstanceError;
use crate::file_manager::{self, Show, Sink};

/// The name the app owns while it runs.
pub const APP_NAME: &str = "org.quire.Files";

/// Whether this app owns the file manager name.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FileManagerName {
    /// Programs that ask the desktop for "the file manager" reach this app.
    Ours,
    /// Another file manager has it; this app does not take it.
    Taken,
}

/// The running app: holds the connection, and with it the names.
#[derive(Debug, Clone)]
pub struct Served {
    file_manager: FileManagerName,
    _connection: Connection,
}

impl Served {
    /// Whether the file manager name is ours.
    pub fn file_manager(&self) -> FileManagerName {
        self.file_manager
    }
}

/// The answer to [`claim`].
#[derive(Debug, Clone)]
pub enum Claim {
    /// This launch is the app. Keep the [`Served`] for as long as it runs.
    First(Served),
    /// Another one is running; [`forward`] this launch's request to it and exit.
    Running,
}

/// A connection to the person's session bus, driven by the runtime the caller is in.
pub async fn session() -> Result<Connection, InstanceError> {
    Ok(Connection::session().await?)
}

/// Becomes the running app if none is, serving each request that arrives at `sink`.
pub async fn claim(connection: &Connection, sink: Sink) -> Result<Claim, InstanceError> {
    // The object goes up before the names, so a call right after the name appears is answered.
    file_manager::serve(connection, sink).await?;
    match ask_for(connection, APP_NAME).await? {
        NameAnswer::Got => {}
        NameAnswer::Taken => {
            file_manager::unserve(connection).await;
            return Ok(Claim::Running);
        }
    }
    let file_manager = match ask_for(connection, file_manager::INTERFACE).await? {
        NameAnswer::Got => FileManagerName::Ours,
        NameAnswer::Taken => FileManagerName::Taken,
    };
    Ok(Claim::First(Served {
        file_manager,
        _connection: connection.clone(),
    }))
}

/// Asks the running app to show `show`.
pub async fn forward(connection: &Connection, show: &Show) -> Result<(), InstanceError> {
    let name = BusName::try_from(APP_NAME).map_err(zbus::Error::from)?;
    let owner = match DBusProxy::new(connection).await?.get_name_owner(name).await {
        Ok(owner) => owner,
        Err(zbus::fdo::Error::NameHasNoOwner(_)) => return Err(InstanceError::NotRunning),
        Err(error) => return Err(zbus::Error::from(error).into()),
    };
    let call = show.to_call();
    connection
        .call_method(
            Some(owner.as_str()),
            file_manager::OBJECT_PATH,
            Some(file_manager::INTERFACE),
            call.method,
            &(call.uris, call.startup_id),
        )
        .await?;
    Ok(())
}

enum NameAnswer {
    Got,
    Taken,
}

/// Asks for `name` without queueing and without replacing its owner.
async fn ask_for(connection: &Connection, name: &'static str) -> Result<NameAnswer, zbus::Error> {
    match connection
        .request_name_with_flags(name, RequestNameFlags::DoNotQueue.into())
        .await
    {
        Ok(RequestNameReply::PrimaryOwner | RequestNameReply::AlreadyOwner) => Ok(NameAnswer::Got),
        Ok(RequestNameReply::Exists | RequestNameReply::InQueue) => Ok(NameAnswer::Taken),
        Err(zbus::Error::NameTaken) => Ok(NameAnswer::Taken),
        Err(error) => Err(error),
    }
}
