//! The launcher: types and default apps from the shared databases, commands from desktop entries.

use std::fmt;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use files_core::{FileName, MimeType};
use files_mime::{Associations, DesktopId, MimeDb, NameGuess};

use super::activate::Activate;
use super::build::{Catalog, LauncherBuilder};
use super::entry::{AppEntry, Start, Terminal};
use super::exec::{ExecLine, Expand, Takes};
use super::icons::AppIcons;
use super::spawn::{Spawn, SpawnCommand};
use super::{AppChoice, LaunchError, Launcher};
use crate::token::ActivationToken;
use crate::uri::file_uri;

/// What a name that says nothing is.
const UNKNOWN_TYPE: &str = "application/octet-stream";

/// Opens files with the installed apps.
pub struct DesktopLauncher {
    pub(super) db: MimeDb,
    pub(super) associations: Associations,
    pub(super) apps: Vec<AppEntry>,
    pub(super) spawn: Arc<dyn Spawn>,
    pub(super) activate: Arc<dyn Activate>,
    pub(super) icons: AppIcons,
}

impl fmt::Debug for DesktopLauncher {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("DesktopLauncher")
            .field("apps", &self.apps.len())
            .finish_non_exhaustive()
    }
}

/// Files for one app.
struct Job<'a> {
    app: &'a AppEntry,
    files: Vec<PathBuf>,
}

impl DesktopLauncher {
    /// A launcher over `catalog`, started apps over `activate` when they are bus activatable.
    /// Further settings are set on the [`LauncherBuilder`] it returns.
    pub fn builder(catalog: Catalog, activate: Arc<dyn Activate>) -> LauncherBuilder {
        LauncherBuilder::new(catalog, activate)
    }

    pub(super) fn app(&self, id: &DesktopId) -> Option<&AppEntry> {
        self.apps.iter().find(|app| app.id == *id)
    }

    /// The type of a file: a folder's, else what its name says, else the type of anything that
    /// has no name to go by. `None` when the path has no usable name.
    pub(super) fn kind_of(&self, target: &Path) -> Option<MimeType> {
        if target.is_dir() {
            return MimeType::new(MimeType::FOLDER).ok();
        }
        let name = FileName::new(target.file_name()?).ok()?;
        match self.db.by_name(&name) {
            NameGuess::Type(kind) => Some(kind),
            NameGuess::Ambiguous(_) | NameGuess::Unknown => MimeType::new(UNKNOWN_TYPE).ok(),
        }
    }

    /// The default app of a file, by its name; `None` when nothing opens it.
    fn default_app(&self, target: &Path) -> Option<&AppEntry> {
        let kind = self.kind_of(target)?;
        let id = self.associations.default_for(&kind, &self.db)?;
        self.app(&id)
    }

    /// Who opens what: each app once, with all its files, in the order they first appear.
    fn plan<'a>(
        &'a self,
        targets: &[PathBuf],
        with: &AppChoice,
    ) -> (Vec<Job<'a>>, Vec<LaunchError>) {
        let mut jobs: Vec<Job<'a>> = Vec::new();
        let mut missing = Vec::new();
        match with {
            AppChoice::App(id) => match self.app(id) {
                Some(app) => jobs.push(Job {
                    app,
                    files: targets.to_vec(),
                }),
                None => missing.push(LaunchError::NoApp(shown(targets.first(), id.as_str()))),
            },
            AppChoice::Default => {
                for target in targets {
                    let Some(app) = self.default_app(target) else {
                        missing.push(LaunchError::NoApp(shown(Some(target), "")));
                        continue;
                    };
                    match jobs.iter_mut().find(|job| job.app.id == app.id) {
                        Some(job) => job.files.push(target.clone()),
                        None => jobs.push(Job {
                            app,
                            files: vec![target.clone()],
                        }),
                    }
                }
            }
        }
        (jobs, missing)
    }

    fn run(
        &self,
        app: &AppEntry,
        files: &[PathBuf],
        token: Option<&ActivationToken>,
    ) -> Result<(), LaunchError> {
        let failed = |reason: String| LaunchError::Failed {
            app: app.name.clone(),
            reason,
        };
        match app.terminal {
            Terminal::Needed => return Err(LaunchError::NeedsTerminal(app.name.clone())),
            Terminal::NotNeeded => {}
        }
        match app.start {
            Start::Bus => {
                let uris: Vec<String> = files.iter().map(|file| file_uri(file)).collect();
                self.activate
                    .open(app.bus_name(), &uris, token)
                    .map_err(|error| failed(error.reason))
            }
            Start::Command => {
                let exec = app.exec.as_deref().unwrap_or_default();
                let line = ExecLine::parse(exec).map_err(|error| failed(error.to_string()))?;
                let none: &[PathBuf] = &[];
                let rounds: Vec<&[PathBuf]> = match line.takes() {
                    Takes::NoFiles => vec![none],
                    Takes::ManyFiles => vec![files],
                    Takes::OneFile if files.is_empty() => vec![none],
                    Takes::OneFile => files.chunks(1).collect(),
                };
                for round in rounds {
                    let argv = line.expand(&Expand {
                        files: round,
                        icon: app.icon.as_deref(),
                        name: &app.name,
                        entry_path: &app.path,
                    });
                    let command = SpawnCommand {
                        argv,
                        env: token_env(token),
                        work_dir: app.work_dir.clone(),
                    };
                    self.spawn
                        .spawn(&command)
                        .map_err(|error| failed(error.reason))?;
                }
                Ok(())
            }
        }
    }
}

/// The variables a started program reads its activation token from.
fn token_env(token: Option<&ActivationToken>) -> Vec<(String, String)> {
    token.map_or_else(Vec::new, |token| {
        ["XDG_ACTIVATION_TOKEN", "DESKTOP_STARTUP_ID"]
            .into_iter()
            .map(|name| (name.to_owned(), token.as_str().to_owned()))
            .collect()
    })
}

/// A file's name for a message; `fallback` when there is no file.
fn shown(path: Option<&PathBuf>, fallback: &str) -> String {
    path.map_or_else(
        || fallback.to_owned(),
        |path| {
            path.file_name().map_or_else(
                || path.to_string_lossy().into_owned(),
                |name| name.to_string_lossy().into_owned(),
            )
        },
    )
}

impl Launcher for DesktopLauncher {
    fn open(
        &self,
        targets: &[PathBuf],
        with: AppChoice,
        token: Option<ActivationToken>,
    ) -> Result<(), LaunchError> {
        let (jobs, mut failures) = self.plan(targets, &with);
        for job in jobs {
            if let Err(error) = self.run(job.app, &job.files, token.as_ref()) {
                failures.push(error);
            }
        }
        failures.into_iter().next().map_or(Ok(()), Err)
    }
}
