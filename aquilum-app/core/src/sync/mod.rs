mod account;
mod apply;
mod conflict;
mod cycle;
mod decide;
pub mod dropbox;
mod halt;
mod measure;
mod oauth;
mod progress;
mod records;
mod remote;
mod scan;
mod service;
mod shared;
mod state;
mod transfer;
pub(crate) use transfer::TRANSFERS;
mod view;
mod worker;

#[cfg(test)]
mod tests;

pub use account::Account;
pub use cycle::{run, Outcome, Vault, Watch, LARGE};
pub use progress::{Count, Progress, Stage};
pub use oauth::{Client, Login, Tokens};
pub use records::{Large, Memory, Records, Size};
pub use remote::{Delta, Remote, RemoteError, RemoteFile, RemoteResult};
pub use service::SyncService;
pub use state::SyncStatus;

use crate::files::error::FileCommandError;
use std::{fmt, io};

#[derive(Debug)]
pub enum SyncError {
    Store(rusqlite::Error),
    Remote(RemoteError),
    File(FileCommandError),
    Changed,
    Deletions { planned: usize, synced: usize },
}

impl fmt::Display for SyncError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Store(error) => write!(formatter, "база синхронизации: {error}"),
            Self::Remote(error) => write!(formatter, "{error}"),
            Self::File(error) => write!(formatter, "{error}"),
            Self::Changed => formatter.write_str("файл изменился во время синхронизации"),
            Self::Deletions { planned, synced } => write!(formatter, "удаляется {planned} из {synced} файлов, синхронизация остановлена"),
        }
    }
}

impl From<rusqlite::Error> for SyncError {
    fn from(error: rusqlite::Error) -> Self {
        Self::Store(error)
    }
}

impl From<RemoteError> for SyncError {
    fn from(error: RemoteError) -> Self {
        Self::Remote(error)
    }
}

impl From<FileCommandError> for SyncError {
    fn from(error: FileCommandError) -> Self {
        Self::File(error)
    }
}

impl From<io::Error> for SyncError {
    fn from(error: io::Error) -> Self {
        Self::File(error.into())
    }
}
