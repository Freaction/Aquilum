use crate::text::content::Content;
use std::fmt;
use std::io::{self, Write};

#[derive(Clone)]
pub struct RemoteFile {
    pub path: String,
    pub id: String,
    pub rev: String,
    pub checksum: String,
    pub size: u64,
    pub modified: u64,
}

pub struct Delta {
    pub changed: Vec<RemoteFile>,
    pub removed: Vec<String>,
    pub cursor: String,
}

#[derive(Debug)]
pub enum RemoteError {
    Auth,
    Stale,
    Missing,
    Failed(String),
}

impl fmt::Display for RemoteError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Auth => formatter.write_str("облако отклонило вход, нужно подключиться заново"),
            Self::Stale => formatter.write_str("файл в облаке изменился во время синхронизации"),
            Self::Missing => formatter.write_str("файла или версии в облаке уже нет"),
            Self::Failed(message) => formatter.write_str(message),
        }
    }
}

impl From<io::Error> for RemoteError {
    fn from(error: io::Error) -> Self {
        Self::Failed(error.to_string())
    }
}

pub type RemoteResult<T> = Result<T, RemoteError>;

pub trait Remote {
    fn cursor(&self) -> RemoteResult<String>;
    fn list(&self) -> RemoteResult<Vec<RemoteFile>>;
    fn delta(&self, cursor: &str) -> RemoteResult<Option<Delta>>;
    fn download(&self, file: &RemoteFile, out: &mut dyn Write) -> RemoteResult<()>;
    fn upload(&self, path: &str, content: &dyn Content, modified: u64, replaces: Option<&RemoteFile>) -> RemoteResult<RemoteFile>;
    fn delete(&self, file: &RemoteFile) -> RemoteResult<()>;
    fn checksum(&self, content: &dyn Content) -> io::Result<String>;
}
