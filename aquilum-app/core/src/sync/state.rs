use super::records::Memory;
use super::{Outcome, Progress, RemoteError, SyncError};
use crate::history::store::millis;
use crate::settings::models::SyncSettings;
use std::path::PathBuf;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use std::thread::JoinHandle;
use std::time::{Instant, SystemTime};

#[derive(Clone, Debug, PartialEq)]
pub enum SyncStatus {
    Off(Option<String>),
    Login,
    Ready,
    Running(Progress),
    Deletions { planned: usize, synced: usize },
    Offline,
    Unsynced { files: usize, first: String },
    Failed(String),
}

#[derive(Default)]
pub struct State {
    pub vault: Option<(PathBuf, SyncSettings, i64)>,
    pub memory: Memory,
    pub running: Option<Progress>,
    pub problem: Option<SyncStatus>,
    pub allow_deletions: bool,
    pub due: Option<Instant>,
    pub epoch: u64,
    pub closed: bool,
    pub login: Option<(Arc<AtomicBool>, JoinHandle<()>)>,
}

impl State {
    pub fn status(&self, connected: bool) -> SyncStatus {
        if self.login.is_some() {
            SyncStatus::Login
        } else if !connected {
            SyncStatus::Off(match &self.problem {
                Some(SyncStatus::Failed(error)) => Some(error.clone()),
                _ => None,
            })
        } else if let Some(progress) = self.running {
            SyncStatus::Running(progress)
        } else {
            self.problem.clone().unwrap_or(SyncStatus::Ready)
        }
    }

    pub fn restart(&mut self) {
        self.epoch += 1;
        (self.memory, self.running, self.problem) = (Memory::default(), None, None);
        self.due = Some(Instant::now());
    }

    pub fn settle(&mut self, passed: Result<Outcome, SyncError>) {
        self.running = None;
        self.problem = match passed {
            Ok(outcome) if outcome.stopped => return,
            Ok(outcome) if outcome.failed.is_empty() => {
                self.memory.done = Some(millis(SystemTime::now()));
                None
            }
            Ok(outcome) => match outcome.failed.iter().find(|(_, error)| !offline(error)) {
                Some((path, error)) => {
                    let first = format!("{path}: {error}").lines().next().unwrap_or_default().to_owned();
                    Some(SyncStatus::Unsynced { files: outcome.failed.len(), first })
                }
                None => Some(SyncStatus::Offline),
            },
            Err(SyncError::Deletions { planned, synced }) => Some(SyncStatus::Deletions { planned, synced }),
            Err(error) if offline(&error) => Some(SyncStatus::Offline),
            Err(error) => Some(SyncStatus::Failed(error.to_string())),
        };
    }
}

fn offline(error: &SyncError) -> bool {
    matches!(error, SyncError::Remote(RemoteError::Offline(_)))
}
