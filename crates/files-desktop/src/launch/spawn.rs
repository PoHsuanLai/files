//! Starting a program, behind a trait so tests record instead of running.

use std::ffi::OsString;
use std::os::unix::process::CommandExt;
use std::path::PathBuf;
use std::process::{Command, Stdio};

use super::StartError;

/// A program to start: argument list (program first), extra environment and folder.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SpawnCommand {
    /// The program and its arguments.
    pub argv: Vec<OsString>,
    /// Environment variables to set, over the app's own.
    pub env: Vec<(String, String)>,
    /// The folder to start it in; the app's own when none.
    pub work_dir: Option<PathBuf>,
}

/// Starts programs.
pub trait Spawn: Send + Sync {
    /// Starts `command` and does not wait for it.
    fn spawn(&self, command: &SpawnCommand) -> Result<(), StartError>;
}

/// Starts real processes: detached, in a group of their own, with no input or output.
#[derive(Debug, Clone, Copy, Default)]
pub struct SystemSpawn;

impl Spawn for SystemSpawn {
    fn spawn(&self, command: &SpawnCommand) -> Result<(), StartError> {
        let (program, args) = command
            .argv
            .split_first()
            .ok_or_else(|| StartError::new("There is nothing to run."))?;
        let mut process = Command::new(program);
        process
            .args(args)
            .envs(command.env.iter().map(|(k, v)| (k.as_str(), v.as_str())))
            .stdin(Stdio::null())
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .process_group(0);
        if let Some(dir) = &command.work_dir {
            process.current_dir(dir);
        }
        let mut child = process
            .spawn()
            .map_err(|error| StartError::new(error.to_string()))?;
        // Reaped when it ends, so it never lingers as a zombie while the app runs.
        drop(std::thread::spawn(move || {
            let _ = child.wait();
        }));
        Ok(())
    }
}

/// Records what it was asked to start, for tests.
#[cfg(feature = "testing")]
#[derive(Debug, Default)]
pub struct FakeSpawn {
    spawned: std::sync::Mutex<Vec<SpawnCommand>>,
    refuse: Option<StartError>,
}

#[cfg(feature = "testing")]
impl FakeSpawn {
    /// A fake that accepts everything.
    pub fn new() -> FakeSpawn {
        FakeSpawn::default()
    }

    /// A fake that refuses everything with `reason`.
    pub fn refusing(reason: &str) -> FakeSpawn {
        FakeSpawn {
            spawned: std::sync::Mutex::default(),
            refuse: Some(StartError::new(reason)),
        }
    }

    /// What was started, in order.
    pub fn spawned(&self) -> Vec<SpawnCommand> {
        self.spawned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .clone()
    }
}

#[cfg(feature = "testing")]
impl Spawn for FakeSpawn {
    fn spawn(&self, command: &SpawnCommand) -> Result<(), StartError> {
        if let Some(error) = &self.refuse {
            return Err(error.clone());
        }
        self.spawned
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner)
            .push(command.clone());
        Ok(())
    }
}
