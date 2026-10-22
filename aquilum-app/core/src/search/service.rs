use super::analysis::graph::GraphSnapshot;
use super::error::SearchError;
use super::guest::Guest;
use super::index::SearchIndex;
use super::models::{IndexRevision, SearchIndexStatus};
use super::paths::{canonical_path, canonical_workspace, same_path};
use super::progress::IndexProgress;
use super::worker::WorkerHandle;
use super::IndexNotifier;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, MutexGuard, RwLock};

#[derive(Clone)]
pub struct SearchService {
    pub base_directory: PathBuf,
    pub active: Arc<RwLock<Option<OpenIndex>>>,
    pub guest: Arc<RwLock<Option<Guest>>>,
    pub prepare_lock: Arc<Mutex<bool>>,
    pub(super) index_notifier: IndexNotifier,
    pub analysis_graph: Arc<RwLock<Option<CachedGraph>>>,
    pub render_lock: Arc<Mutex<()>>,
    pub next_render_epoch: Arc<AtomicU64>,
    pub(super) next_generation: Arc<AtomicU64>,
}

pub struct CachedGraph {
    pub root: PathBuf,
    pub generation: u64,
    pub revision: u64,
    pub graph: Arc<GraphSnapshot>,
}

pub struct OpenIndex {
    pub root: PathBuf,
    pub index: Arc<SearchIndex>,
    pub(super) worker: WorkerHandle,
    pub progress: Arc<IndexProgress>,
    pub metadata_path: PathBuf,
    pub generation: u64,
    pub(super) visible: Arc<AtomicBool>,
}

impl OpenIndex {
    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Relaxed)
    }

    fn queue(&self, paths: &[PathBuf], rescan: bool) {
        if rescan {
            return self.worker.queue_rescan();
        }
        let inside = paths.iter().filter(|path| path.starts_with(&self.root)).cloned().collect::<Vec<_>>();
        if !inside.is_empty() {
            self.worker.queue_paths(inside);
        }
    }
}

impl SearchService {
    pub fn new(
        base_directory: PathBuf,
        index_notifier: IndexNotifier,
    ) -> Self {
        Self {
            base_directory,
            active: Arc::new(RwLock::new(None)),
            guest: Arc::new(RwLock::new(None)),
            prepare_lock: Arc::new(Mutex::new(false)),
            index_notifier,
            analysis_graph: Arc::new(RwLock::new(None)),
            render_lock: Arc::new(Mutex::new(())),
            next_render_epoch: Arc::new(AtomicU64::new(0)),
            next_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub(super) fn opening(&self) -> Result<MutexGuard<'_, bool>, SearchError> {
        let closed = self.prepare_lock.lock().map_err(SearchError::task)?;
        if *closed {
            return Err(SearchError::Unavailable { message: "Search is closed".to_owned() });
        }
        Ok(closed)
    }

    pub fn prepare(&self, workspace: &str) -> Result<SearchIndexStatus, SearchError> {
        let _opening = self.opening()?;
        let root = workspace_root(workspace)?;
        if let Some(status) = self.status_for_root(&root) {
            return Ok(status);
        }
        let opened = match self.take_guest(&root)? {
            Some(guest) => {
                guest.visible.store(true, Ordering::Relaxed);
                guest
            }
            None => self.open_index(root, true)?,
        };
        let progress = Arc::clone(&opened.progress);
        let generation = opened.generation;
        let previous = self.active.write().map_err(SearchError::task)?.replace(opened);
        drop(previous);
        clear_if(&self.analysis_graph, |_| true);
        let snapshot = progress.snapshot();
        (self.index_notifier)(IndexRevision {
            workspace_path: workspace.to_owned(),
            generation,
            revision: snapshot.revision,
        });
        Ok(snapshot)
    }

    pub fn forget_analysis(&self) {
        clear_if(&self.analysis_graph, |_| true);
    }

    pub fn queue(&self, paths: &[PathBuf], rescan: bool) {
        let canonical = paths.iter().map(|path| canonical_path(path)).collect::<Vec<_>>();
        if let Some(active) = self.active.read().ok().as_deref().and_then(Option::as_ref) {
            active.queue(&canonical, rescan);
        }
        if let Some(guest) = self.guest.read().ok().as_deref().and_then(Option::as_ref) {
            guest.index.queue(&canonical, rescan);
        }
    }

    pub fn status_for_root(&self, root: &Path) -> Option<SearchIndexStatus> {
        self.active.read().ok().and_then(|active| {
            active
                .as_ref()
                .filter(|value| same_path(&value.root, root))
                .map(|value| value.progress.snapshot())
        })
    }
}

pub(super) fn clear_if<T>(cache: &RwLock<Option<T>>, stale: impl Fn(&T) -> bool) {
    if let Ok(mut cached) = cache.write() {
        if cached.as_ref().is_some_and(stale) {
            *cached = None;
        }
    }
}

pub fn workspace_root(workspace: &str) -> Result<PathBuf, SearchError> {
    let root = canonical_workspace(workspace)?;
    if !root.is_dir() {
        return Err(SearchError::InvalidWorkspace {
            message: format!("Not a directory: {}", root.display()),
        });
    }
    Ok(root)
}
