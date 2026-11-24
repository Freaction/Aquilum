use super::error::SearchError;
use super::models::{SearchIndexState, SearchIndexStatus};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Mutex;

pub struct IndexProgress {
    generation: u64,
    revision: AtomicU64,
    ready: AtomicBool,
    updating: AtomicBool,
    indexed: AtomicU64,
    scanned: AtomicU64,
    error: Mutex<Option<String>>,
}

impl IndexProgress {
    pub fn new(indexed: u64, generation: u64) -> Self {
        Self {
            generation,
            revision: AtomicU64::new(0),
            ready: AtomicBool::new(indexed > 0),
            updating: AtomicBool::new(true),
            indexed: AtomicU64::new(indexed),
            scanned: AtomicU64::new(0),
            error: Mutex::new(None),
        }
    }

    pub fn snapshot(&self) -> SearchIndexStatus {
        let error = self.error.lock().ok().and_then(|value| value.clone());
        let ready = self.ready.load(Ordering::Relaxed);
        SearchIndexStatus {
            state: if error.is_some() && !ready {
                SearchIndexState::Error
            } else if ready {
                SearchIndexState::Ready
            } else {
                SearchIndexState::Indexing
            },
            updating: self.updating.load(Ordering::Relaxed),
            generation: self.generation,
            revision: self.revision.load(Ordering::Relaxed),
            indexed_documents: self.indexed.load(Ordering::Relaxed),
            scanned_documents: self.scanned.load(Ordering::Relaxed),
            error,
        }
    }

    pub fn bump_revision(&self) {
        self.revision.fetch_add(1, Ordering::Relaxed);
    }

    pub fn begin_scan(&self) {
        self.updating.store(true, Ordering::Relaxed);
        self.scanned.store(0, Ordering::Relaxed);
    }

    pub fn scanned_one(&self) -> u64 {
        self.scanned.fetch_add(1, Ordering::Relaxed) + 1
    }

    pub fn committed(&self, count: u64) {
        self.indexed.store(count, Ordering::Relaxed);
        self.ready.store(true, Ordering::Relaxed);
    }

    pub fn completed(&self, count: u64) {
        self.committed(count);
        self.updating.store(false, Ordering::Relaxed);
        if let Ok(mut error) = self.error.lock() {
            *error = None;
        }
    }

    pub fn failed(&self, error: &SearchError) {
        self.updating.store(false, Ordering::Relaxed);
        eprintln!("[aquilum:index] failed: {error:?}");
        if let Ok(mut message) = self.error.lock() {
            *message = Some(format!("{error:?}"));
        }
    }
}
