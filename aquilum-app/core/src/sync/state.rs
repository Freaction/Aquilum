use super::records::Memory;
use super::{Outcome, Progress, SyncError};
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
    Failed(String),
}

#[derive(Default)]
pub struct State {
    pub vault: Option<(PathBuf, SyncSettings, i64)>,
    pub memory: Memory,
    pub running: Option<Progress>,
    pub error: Option<String>,
    pub deletions: Option<(usize, usize)>,
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
            SyncStatus::Off(self.error.clone())
        } else if let Some(progress) = self.running {
            SyncStatus::Running(progress)
        } else if let Some((planned, synced)) = self.deletions {
            SyncStatus::Deletions { planned, synced }
        } else if let Some(error) = &self.error {
            SyncStatus::Failed(error.clone())
        } else {
            SyncStatus::Ready
        }
    }

    pub fn restart(&mut self) {
        self.epoch += 1;
        (self.memory, self.running, self.error, self.deletions) = (Memory::default(), None, None, None);
        self.due = Some(Instant::now());
    }

    pub fn settle(&mut self, passed: Result<Outcome, SyncError>) {
        self.running = None;
        (self.error, self.deletions) = match passed {
            Ok(outcome) if outcome.stopped => return,
            Ok(outcome) if outcome.failed.is_empty() => {
                self.memory.done = Some(millis(SystemTime::now()));
                (None, None)
            }
            Ok(outcome) => (Some(outcome.failed.iter().map(|(path, error)| format!("{path}: {error}")).collect::<Vec<_>>().join("\n")), None),
            Err(SyncError::Deletions { planned, synced }) => (None, Some((planned, synced))),
            Err(error) => (Some(error.to_string()), None),
        };
    }
}
