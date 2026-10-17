use crate::text::content::Content;
use super::conflict::{copy_content, copy_stem, ConflictReport, LocalMinute};
use super::resolve::{SyncPoint, SyncPointLookup};
use super::session::{DiskError, DiskSnapshot, SessionIo};
use super::store::DocumentStore;
use crate::files::document::{hash_bytes, hash_file, read_file_snapshot_impl};
use crate::files::entries::first_free;
use crate::files::error::FileCommandError;
use crate::files::gate;
use crate::history::Source;
use std::path::{Path, PathBuf};
use std::sync::Mutex;
use std::time::{SystemTime, UNIX_EPOCH};
use crate::app_core::Core;


pub struct HubIo<'a> {
    pub core: &'a Core,
    pub path: &'a Path,
    pub key: &'a str,
    pub store: &'a Mutex<Option<DocumentStore>>,
    pub offset_seconds: i64,
    pub source: Source,
}

fn disk_error(path: &Path, error: FileCommandError) -> DiskError {
    match error {
        FileCommandError::Conflict { .. } => DiskError::Conflict,
        _ if !path.exists() => DiskError::Missing,
        FileCommandError::InvalidUtf8 { message } => DiskError::Other { code: "invalid_utf8", message },
        error => DiskError::Other { code: "io", message: error.to_string() },
    }
}

fn now_nanos() -> i64 {
    SystemTime::now().duration_since(UNIX_EPOCH).map_or(0, |since| since.as_nanos() as i64)
}

fn copy_path(note: &Path, label: &str, attempt: usize) -> PathBuf {
    let name = if attempt == 0 { format!("{label}.md") } else { format!("{label} {}.md", attempt + 1) };
    note.with_file_name(name)
}

pub fn preserve_copy(core: &Core, path: &Path, body: &dyn Content, report: ConflictReport, offset_seconds: i64) -> Option<PathBuf> {
    let stem = crate::search::paths::title(path);
    let moment = LocalMinute::at(now_nanos(), offset_seconds);
    let note = path.to_string_lossy();
    let content = copy_content(&note, &stem, &report, &moment, body)?;
    let label = copy_stem(&stem, &moment);
    first_free(|attempt| copy_path(path, &label, attempt), |target| gate::create(core, target, &content, Source::Me, None))
        .map_err(|error| eprintln!("[aquilum:documents] копия-конфликт {label} не записана: {error}"))
        .ok()
}

impl HubIo<'_> {
    fn with_store<T>(&self, operation: impl FnOnce(&DocumentStore) -> T) -> Option<T> {
        let guard = self.store.lock().ok()?;
        guard.as_ref().map(operation)
    }
}

impl SessionIo for HubIo<'_> {
    fn read(&self) -> Result<DiskSnapshot, DiskError> {
        let snapshot = read_file_snapshot_impl(self.path).map_err(|error| disk_error(self.path, error))?;
        Ok(DiskSnapshot { content: snapshot.content, hash: snapshot.hash, text_hash: snapshot.text_hash })
    }

    fn hash(&self) -> Result<String, DiskError> {
        hash_file(self.path).map_err(|error| disk_error(self.path, error.into()))
    }

    fn write(&self, content: &str, expected_hash: &str) -> Result<String, DiskError> {
        gate::write(self.core, self.path, content, Some(expected_hash), self.source, None)
            .map(|result| result.hash)
            .map_err(|error| disk_error(self.path, error))
    }

    fn text_hash(&self, text: &str) -> String {
        hash_bytes(text.as_bytes())
    }

    fn preserve(&self, body: &str, report: ConflictReport) {
        preserve_copy(self.core, self.path, &body, report, self.offset_seconds);
    }

    fn append(&self, update: &[u8]) {
        match self.with_store(|store| store.append(self.key, update)) {
            Some(Err(error)) => eprintln!("[aquilum:documents] правка не сохранена в локальную копию: {error}"),
            None => eprintln!("[aquilum:documents] локальная копия документов недоступна"),
            Some(Ok(())) => {}
        }
    }

    fn sync_point(&self) -> SyncPointLookup {
        self.with_store(|store| store.sync_point(self.key)).unwrap_or(SyncPointLookup::Unavailable)
    }

    fn remember(&self, point: SyncPoint) {
        if let Some(Err(error)) = self.with_store(|store| store.remember_sync_point(self.key, &point)) {
            eprintln!("[aquilum:documents] точка синхронизации не сохранена: {error}");
        }
    }
}
