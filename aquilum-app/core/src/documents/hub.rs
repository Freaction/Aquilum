use super::io::HubIo;
use super::session::{DocumentSession, LoggedChange, Pull, SessionError, SYSTEM_CLIENT};
use super::store::DocumentStore;
use crate::history::Source;
use crate::search::paths::identity;
use super::resolve::SyncPoint;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicI64, Ordering};
use std::sync::mpsc::{self, Receiver, RecvTimeoutError, Sender};
use std::sync::Mutex;
use std::time::{Duration, Instant};
use crate::app_core::{Core, CoreEvent};
use std::sync::Weak;


#[derive(Debug)]
pub enum HubError {
    NotOpen,
    Stale,
    Session(SessionError),
}

impl From<SessionError> for HubError {
    fn from(error: SessionError) -> Self {
        Self::Session(error)
    }
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OpenedDocument {
    pub text: String,
    pub version: u64,
}

#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "camelCase")]
pub enum PullResult {
    Changes { changes: Vec<LoggedChange> },
    Resync { text: String, version: u64 },
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentEvent {
    pub path: String,
    pub version: u64,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveFailed {
    pub path: String,
    pub reason: String,
}

struct Entry {
    path: PathBuf,
    session: DocumentSession,
    holders: usize,
    moving: bool,
}

fn inside(key: &str, folder_key: &str) -> bool {
    key == folder_key || key.strip_prefix(folder_key).is_some_and(|rest| rest.starts_with('/'))
}

pub struct DocumentHub {
    store: Mutex<Option<DocumentStore>>,
    sessions: Mutex<HashMap<String, Entry>>,
    writes: Mutex<Option<Sender<(String, Instant)>>>,
    offset_seconds: AtomicI64,
}

impl DocumentHub {
    pub fn new(path: &Path) -> Self {
        let store = DocumentStore::open(path)
            .map_err(|error| eprintln!("[aquilum:documents] локальная копия документов не открылась: {error}"))
            .ok();
        Self {
            store: Mutex::new(store),
            sessions: Mutex::new(HashMap::new()),
            writes: Mutex::new(None),
            offset_seconds: AtomicI64::new(0),
        }
    }

    pub fn start(&self, core: Weak<Core>) {
        let (sender, receiver) = mpsc::channel();
        std::thread::spawn(move || run_writes(core, receiver));
        if let Ok(mut writes) = self.writes.lock() {
            *writes = Some(sender);
        }
    }

    fn io<'a>(&'a self, core: &'a Core, path: &'a Path, key: &'a str) -> HubIo<'a> {
        self.io_as(core, path, key, Source::Me)
    }

    fn io_as<'a>(&'a self, core: &'a Core, path: &'a Path, key: &'a str, source: Source) -> HubIo<'a> {
        HubIo {
            core,
            path,
            key,
            store: &self.store,
            offset_seconds: self.offset_seconds.load(Ordering::Relaxed),
            source,
        }
    }

    pub fn open(&self, core: &Core, path: &Path, offset_minutes: i64) -> Result<OpenedDocument, HubError> {
        self.offset_seconds.store(offset_minutes * 60, Ordering::Relaxed);
        let key = identity(path);
        let mut sessions = self.sessions.lock().expect("document sessions");
        if let Some(entry) = sessions.get_mut(&key) {
            entry.holders += 1;
            let path = entry.path.clone();
            self.reconcile_entry(core, &key, &path, entry);
            return Ok(OpenedDocument { text: entry.session.text(), version: entry.session.version() });
        }
        let blobs = self.load(&key);
        let session = DocumentSession::open(&blobs, &self.io(core, path, &key))?;
        self.compact(&key, &session);
        if session.dirty() {
            self.schedule(&key, Duration::ZERO);
        }
        let opened = OpenedDocument { text: session.text(), version: session.version() };
        sessions.insert(key, Entry { path: path.to_path_buf(), session, holders: 1, moving: false });
        Ok(opened)
    }

    pub fn push(
        &self,
        core: &Core,
        path: &Path,
        version: u64,
        client: &str,
        changes: &[Value],
    ) -> Result<bool, HubError> {
        let key = identity(path);
        let mut sessions = self.sessions.lock().expect("document sessions");
        let entry = sessions.get_mut(&key).ok_or(HubError::NotOpen)?;
        let entry_path = entry.path.clone();
        let accepted = entry.session.push(client, version, changes, &self.io(core, &entry_path, &key))?;
        if accepted {
            self.schedule(&key, self.save_delay(core));
            announce_change(core, &entry_path, entry.session.version());
        }
        Ok(accepted)
    }

    pub fn pull(&self, path: &Path, version: u64) -> Result<PullResult, HubError> {
        let sessions = self.sessions.lock().expect("document sessions");
        let entry = sessions.get(&identity(path)).ok_or(HubError::NotOpen)?;
        Ok(match entry.session.pull(version) {
            Pull::Changes(changes) => PullResult::Changes { changes },
            Pull::Resync => PullResult::Resync { text: entry.session.text(), version: entry.session.version() },
        })
    }

    pub fn encode_positions(&self, path: &Path, indices: &[usize]) -> Option<Vec<Option<Vec<u8>>>> {
        let sessions = self.sessions.lock().expect("document sessions");
        let entry = sessions.get(&identity(path))?;
        Some(indices.iter().map(|index| entry.session.encode_position(*index)).collect())
    }

    pub fn resolve_positions(&self, path: &Path, positions: &[Vec<u8>]) -> Result<Vec<Option<usize>>, HubError> {
        let sessions = self.sessions.lock().expect("document sessions");
        let entry = sessions.get(&identity(path)).ok_or(HubError::NotOpen)?;
        Ok(positions.iter().map(|encoded| entry.session.resolve_position(encoded)).collect())
    }

    pub fn read(&self, core: &Core, path: &Path) -> Result<OpenedDocument, HubError> {
        self.with_document(core, path, |_, entry| {
            Ok(OpenedDocument { text: entry.session.text(), version: entry.session.version() })
        })
    }

    pub fn replace_text(
        &self,
        core: &Core,
        path: &Path,
        text: &str,
        client: &str,
        source: Source,
        base_version: Option<u64>,
    ) -> Result<u64, HubError> {
        self.with_document(core, path, |key, entry| {
            if base_version.is_some_and(|base| base != entry.session.version()) {
                return Err(HubError::Stale);
            }
            let entry_path = entry.path.clone();
            let before = entry.session.version();
            entry.session.replace_text(text, client, &self.io(core, &entry_path, key));
            if entry.session.version() != before {
                announce_change(core, &entry_path, entry.session.version());
            }
            self.write_entry_as(core, key, entry, source);
            Ok(entry.session.version())
        })
    }

    pub fn revert(
        &self,
        core: &Core,
        path: &Path,
        version_text: &str,
        previous_text: &str,
        source: Source,
    ) -> Result<bool, HubError> {
        self.with_document(core, path, |key, entry| {
            let entry_path = entry.path.clone();
            let changed =
                entry.session.revert(version_text, previous_text, SYSTEM_CLIENT, &self.io(core, &entry_path, key));
            if changed {
                announce_change(core, &entry_path, entry.session.version());
                self.write_entry_as(core, key, entry, source);
            }
            Ok(changed)
        })
    }

    fn with_document<T>(
        &self,
        core: &Core,
        path: &Path,
        operation: impl FnOnce(&str, &mut Entry) -> Result<T, HubError>,
    ) -> Result<T, HubError> {
        let opened_here = !self.is_open(path);
        if opened_here {
            self.open(core, path, self.offset_seconds.load(Ordering::Relaxed) / 60)?;
        }
        let key = identity(path);
        let result = {
            let mut sessions = self.sessions.lock().expect("document sessions");
            match sessions.get_mut(&key) {
                Some(entry) => operation(&key, entry),
                None => Err(HubError::NotOpen),
            }
        };
        if opened_here {
            self.release(core, path);
        }
        result
    }

    pub fn import_legacy(&self, replicas: &[LegacyReplica]) -> usize {
        let store = self.store.lock().expect("document store");
        let Some(store) = store.as_ref() else { return 0 };
        replicas.iter().filter(|replica| import_one(store, replica)).count()
    }

    pub fn release(&self, core: &Core, path: &Path) {
        let key = identity(path);
        let mut sessions = self.sessions.lock().expect("document sessions");
        let Some(entry) = sessions.get_mut(&key) else { return };
        entry.holders = entry.holders.saturating_sub(1);
        if entry.holders > 0 {
            return;
        }
        let mut entry = sessions.remove(&key).expect("entry exists");
        self.write_entry(core, &key, &mut entry);
        self.compact(&key, &entry.session);
    }

    pub fn flush_all(&self, core: &Core) {
        let mut sessions = self.sessions.lock().expect("document sessions");
        for (key, entry) in sessions.iter_mut() {
            self.write_entry(core, key, entry);
        }
    }

    pub fn reconcile_paths(&self, core: &Core, paths: &[PathBuf]) {
        let mut sessions = self.sessions.lock().expect("document sessions");
        for path in paths {
            let key = identity(path);
            if let Some(entry) = sessions.get_mut(&key) {
                let entry_path = entry.path.clone();
                self.reconcile_entry(core, &key, &entry_path, entry);
            }
        }
    }

    pub fn reconcile_all(&self, core: &Core) {
        let mut sessions = self.sessions.lock().expect("document sessions");
        for (key, entry) in sessions.iter_mut() {
            let entry_path = entry.path.clone();
            self.reconcile_entry(core, key, &entry_path, entry);
        }
    }

    pub fn relocated(&self, moves: &[(String, String)], removed: &[String]) {
        let mut sessions = self.sessions.lock().expect("document sessions");
        let mut store = self.store.lock().expect("document store");
        for (from, to) in moves {
            let (from_key, to_key) = (identity(Path::new(from)), identity(Path::new(to)));
            if let Some(mut entry) = sessions.remove(&from_key) {
                entry.path = PathBuf::from(to);
                entry.moving = false;
                sessions.insert(to_key.clone(), entry);
            }
            if let Some(Err(error)) = store.as_mut().map(|store| store.relocate(&from_key, &to_key)) {
                eprintln!("[aquilum:documents] локальная копия не перенесена: {error}");
            }
        }
        for path in removed {
            let key = identity(Path::new(path));
            sessions.remove(&key);
            if let Some(Err(error)) = store.as_mut().map(|store| store.forget(&key)) {
                eprintln!("[aquilum:documents] локальная копия удалённой заметки не стёрта: {error}");
            }
        }
    }

    pub fn moving<T>(&self, core: &Core, from: &Path, operation: impl FnOnce() -> T) -> T {
        let folder_key = identity(from);
        {
            let mut sessions = self.sessions.lock().expect("document sessions");
            for (key, entry) in sessions.iter_mut().filter(|(key, _)| inside(key, &folder_key)) {
                self.write_entry(core, key, entry);
                entry.moving = true;
            }
        }
        let result = operation();
        let mut sessions = self.sessions.lock().expect("document sessions");
        for (key, entry) in sessions.iter_mut().filter(|(_, entry)| entry.moving) {
            entry.moving = false;
            if entry.session.dirty() {
                self.schedule(key, Duration::ZERO);
            }
        }
        result
    }

    fn write_due(&self, core: &Core, key: &str) {
        let mut sessions = self.sessions.lock().expect("document sessions");
        if let Some(entry) = sessions.get_mut(key) {
            self.write_entry(core, key, entry);
        }
    }

    fn is_open(&self, path: &Path) -> bool {
        self.sessions.lock().expect("document sessions").contains_key(&identity(path))
    }

    fn reconcile_entry(&self, core: &Core, key: &str, path: &Path, entry: &mut Entry) {
        let before = entry.session.version();
        match entry.session.reconcile(&self.io(core, path, key)) {
            Ok(()) => {}
            Err(SessionError::Missing) => announce_missing(core, path),
            Err(error) => eprintln!("[aquilum:documents] сверка с диском не удалась: {error}"),
        }
        if entry.session.version() != before {
            announce_change(core, path, entry.session.version());
        }
        if entry.session.dirty() {
            self.schedule(key, Duration::ZERO);
        }
    }

    fn write_entry(&self, core: &Core, key: &str, entry: &mut Entry) {
        self.write_entry_as(core, key, entry, Source::Me);
    }

    fn write_entry_as(&self, core: &Core, key: &str, entry: &mut Entry, source: Source) {
        if !entry.session.dirty() || entry.moving {
            return;
        }
        let path = entry.path.clone();
        let before = entry.session.version();
        let result = entry.session.write(&self.io_as(core, &path, key, source));
        if entry.session.version() != before {
            announce_change(core, &path, entry.session.version());
        }
        let path_text = path.to_string_lossy().into_owned();
        core.emit(match result {
            Ok(()) => CoreEvent::DocumentSaved(DocumentEvent { path: path_text, version: entry.session.version() }),
            Err(SessionError::Missing) => CoreEvent::DocumentMissing(DocumentEvent { path: path_text, version: 0 }),
            Err(error) => CoreEvent::DocumentSaveFailed(SaveFailed { path: path_text, reason: error.to_string() }),
        });
    }

    fn load(&self, key: &str) -> Vec<Vec<u8>> {
        let store = self.store.lock().expect("document store");
        match store.as_ref().map(|store| store.load(key)) {
            Some(Ok(blobs)) => blobs,
            Some(Err(error)) => {
                eprintln!("[aquilum:documents] локальная копия не прочиталась, документ открыт с диска: {error}");
                Vec::new()
            }
            None => Vec::new(),
        }
    }

    fn compact(&self, key: &str, session: &DocumentSession) {
        let mut store = self.store.lock().expect("document store");
        if let Some(Err(error)) = store.as_mut().map(|store| store.compact(key, &session.state())) {
            eprintln!("[aquilum:documents] локальная копия не сжата: {error}");
        }
    }

    fn save_delay(&self, core: &Core) -> Duration {
        Duration::from_millis(u64::from(core.settings.get_config().editor.save_debounce_ms))
    }

    fn schedule(&self, key: &str, delay: Duration) {
        let writes = self.writes.lock().expect("document writes");
        if let Some(sender) = writes.as_ref() {
            let _ = sender.send((key.to_owned(), Instant::now() + delay));
        }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyReplica {
    pub path: String,
    pub updates: Vec<Vec<u8>>,
    pub point: Option<LegacyPoint>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LegacyPoint {
    pub file_hash: String,
    pub text_hash: String,
}

fn import_one(store: &DocumentStore, replica: &LegacyReplica) -> bool {
    let path = Path::new(&replica.path);
    let key = identity(path);
    if replica.updates.is_empty() || !path.is_file() || store.has_replica(&key).unwrap_or(true) {
        return false;
    }
    let appended = replica.updates.iter().try_for_each(|update| store.append(&key, update));
    let remembered = replica.point.as_ref().map_or(Ok(()), |point| {
        store.remember_sync_point(&key, &SyncPoint { file_hash: point.file_hash.clone(), text_hash: point.text_hash.clone() })
    });
    match appended.and(remembered) {
        Ok(()) => true,
        Err(error) => {
            eprintln!("[aquilum:documents] старая локальная копия {} не перенесена: {error}", replica.path);
            false
        }
    }
}

fn announce_change(core: &Core, path: &Path, version: u64) {
    core.emit(CoreEvent::DocumentChanged(DocumentEvent { path: path.to_string_lossy().into_owned(), version }));
}

fn announce_missing(core: &Core, path: &Path) {
    core.emit(CoreEvent::DocumentMissing(DocumentEvent { path: path.to_string_lossy().into_owned(), version: 0 }));
}

fn run_writes(core: Weak<Core>, receiver: Receiver<(String, Instant)>) {
    let mut pending: HashMap<String, Instant> = HashMap::new();
    loop {
        let received = match pending.values().min() {
            Some(at) => receiver.recv_timeout(at.saturating_duration_since(Instant::now())),
            None => receiver.recv().map_err(|_| RecvTimeoutError::Disconnected),
        };
        match received {
            Ok((key, at)) => {
                let due = pending.entry(key).or_insert(at);
                *due = (*due).min(at);
            }
            Err(RecvTimeoutError::Timeout) => {}
            Err(RecvTimeoutError::Disconnected) => return,
        }
        let now = Instant::now();
        let due: Vec<String> = pending.iter().filter(|(_, at)| **at <= now).map(|(key, _)| key.clone()).collect();
        if due.is_empty() {
            continue;
        }
        let Some(core) = core.upgrade() else { return };
        for key in due {
            pending.remove(&key);
            core.documents.write_due(&core, &key);
        }
    }
}

#[cfg(test)]
#[path = "hub_tests.rs"]
mod tests;
