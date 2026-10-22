use super::analysis::graph::GraphSnapshot;
use super::error::SearchError;
use super::guest::Guest;
use super::index::SearchIndex;
use super::metadata;
use super::models::{IndexRevision, SearchIndexStatus, SearchResponse};
use super::paths::{canonical_path, canonical_workspace, same_path};
use super::progress::IndexProgress;
use super::query::Request;
use super::worker::WorkerHandle;
use super::IndexNotifier;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::{Arc, Mutex, RwLock};

#[derive(Clone)]
pub struct SearchService {
    pub base_directory: PathBuf,
    pub active: Arc<RwLock<Option<OpenIndex>>>,
    pub guest: Arc<RwLock<Option<Guest>>>,
    pub prepare_lock: Arc<Mutex<()>>,
    index_notifier: IndexNotifier,
    pub analysis_graph: Arc<RwLock<Option<CachedGraph>>>,
    pub render_lock: Arc<Mutex<()>>,
    pub next_render_epoch: Arc<AtomicU64>,
    next_generation: Arc<AtomicU64>,
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
    worker: WorkerHandle,
    pub progress: Arc<IndexProgress>,
    pub metadata_path: PathBuf,
    pub generation: u64,
    visible: Arc<AtomicBool>,
}

impl OpenIndex {
    pub fn is_visible(&self) -> bool {
        self.visible.load(Ordering::Relaxed)
    }

    pub fn stop(self) {
        self.worker.stop();
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
            prepare_lock: Arc::new(Mutex::new(())),
            index_notifier,
            analysis_graph: Arc::new(RwLock::new(None)),
            render_lock: Arc::new(Mutex::new(())),
            next_render_epoch: Arc::new(AtomicU64::new(0)),
            next_generation: Arc::new(AtomicU64::new(0)),
        }
    }

    pub fn prepare(&self, workspace: &str) -> Result<SearchIndexStatus, SearchError> {
        let _guard = self.prepare_lock.lock().map_err(SearchError::task)?;
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
        if let Some(previous) = previous {
            previous.stop();
        }
        clear_if(&self.analysis_graph, |_| true);
        let snapshot = progress.snapshot();
        (self.index_notifier)(IndexRevision {
            workspace_path: workspace.to_owned(),
            generation,
            revision: snapshot.revision,
        });
        Ok(snapshot)
    }

    pub fn open_index(&self, root: PathBuf, visible: bool) -> Result<OpenIndex, SearchError> {
        fs::create_dir_all(&self.base_directory)?;
        let key = blake3::hash(root.to_string_lossy().as_bytes()).to_hex();
        let storage = self.base_directory.join(key.as_str());
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let index_dir = storage.join(format!("index-v{}", crate::search::ANALYZER_VERSION));
        let index = Arc::new(SearchIndex::open(&index_dir)?);
        remove_stale_indexes(&storage, &index_dir);
        let progress = Arc::new(IndexProgress::new(index.document_count(), generation));
        let metadata_path = storage.join("documents.sqlite3");
        drop(metadata::open(&metadata_path)?);
        let visible = Arc::new(AtomicBool::new(visible));
        let worker = WorkerHandle::spawn(
            root.clone(),
            Arc::clone(&index),
            metadata_path.clone(),
            Arc::clone(&progress),
            generation,
            self.refresh_notifier(root.clone(), generation, Arc::clone(&visible)),
        )?;
        Ok(OpenIndex {
            root,
            index,
            worker,
            progress,
            metadata_path,
            generation,
            visible,
        })
    }

    fn refresh_notifier(
        &self,
        root: PathBuf,
        generation: u64,
        visible: Arc<AtomicBool>,
    ) -> IndexNotifier {
        let analysis_graph = Arc::clone(&self.analysis_graph);
        let index_notifier = Arc::clone(&self.index_notifier);
        Arc::new(move |event: IndexRevision| {
            clear_if(&analysis_graph, |cached| {
                cached.root == root && cached.generation == generation
            });
            if visible.load(Ordering::Relaxed) {
                (index_notifier)(event);
            }
        })
    }

    pub fn forget_analysis(&self) {
        clear_if(&self.analysis_graph, |_| true);
    }

    pub fn search(&self, workspace: &str, request: &Request) -> Result<SearchResponse, SearchError> {
        let found = self.with_index(workspace, |open| {
            Ok((Arc::clone(&open.index), Arc::clone(&open.progress), open.metadata_path.clone()))
        });
        let (index, progress, metadata) = match found {
            Ok(found) => found,
            Err(SearchError::Unavailable { .. }) => return Ok(SearchResponse::idle()),
            Err(error) => return Err(error),
        };
        let found = index.search(&Request { metadata: Some(&metadata), cancel: request.cancel.clone(), ..*request })?;
        Ok(SearchResponse {
            query_terms: found.terms,
            results: found.results,
            has_more: found.has_more,
            status: progress.snapshot(),
        })
    }

    pub fn with_index<T>(
        &self,
        workspace: &str,
        read: impl FnOnce(&OpenIndex) -> Result<T, SearchError>,
    ) -> Result<T, SearchError> {
        let root = canonical_workspace(workspace).map_err(|_| self.missing_index())?;
        self.locate(|open| same_path(&open.root, &root), read)?
            .ok_or_else(|| self.missing_index())
    }

    pub fn with_index_containing<T>(
        &self,
        path: &Path,
        read: impl FnOnce(&OpenIndex) -> Result<T, SearchError>,
    ) -> Result<Option<T>, SearchError> {
        self.locate(|open| path.starts_with(&open.root), read)
    }

    fn locate<T>(
        &self,
        fits: impl Fn(&OpenIndex) -> bool,
        read: impl FnOnce(&OpenIndex) -> Result<T, SearchError>,
    ) -> Result<Option<T>, SearchError> {
        {
            let active = self.active.read().map_err(SearchError::task)?;
            if let Some(open) = active.as_ref().filter(|open| fits(open)) {
                return read(open).map(Some);
            }
        }
        let guest = self.guest.read().map_err(SearchError::task)?;
        match guest.as_ref().filter(|guest| fits(&guest.index)) {
            Some(guest) => {
                guest.touch();
                read(&guest.index).map(Some)
            }
            None => Ok(None),
        }
    }

    fn missing_index(&self) -> SearchError {
        if self.active_root().is_some() {
            SearchError::invalid_workspace("Index belongs to a different workspace")
        } else {
            SearchError::Unavailable {
                message: "Index is not ready".to_owned(),
            }
        }
    }

    pub fn index_paths(&self, workspace: &str) -> Result<(PathBuf, PathBuf), SearchError> {
        let paths = |open: &OpenIndex| Ok((open.root.clone(), open.metadata_path.clone()));
        if let Ok(found) = self.with_index(workspace, paths) {
            return Ok(found);
        }
        self.prepare(workspace)?;
        self.with_index(workspace, paths)
    }

    pub fn active_root(&self) -> Option<PathBuf> {
        self.active
            .read()
            .ok()
            .and_then(|active| active.as_ref().map(|value| value.root.clone()))
    }

    pub fn status(&self, workspace: Option<&str>) -> SearchIndexStatus {
        match workspace {
            Some(workspace) => self
                .with_index(workspace, |open| Ok(open.progress.snapshot()))
                .ok(),
            None => self
                .active
                .read()
                .ok()
                .and_then(|active| active.as_ref().map(|open| open.progress.snapshot())),
        }
        .unwrap_or_else(SearchIndexStatus::idle)
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

fn clear_if<T>(cache: &RwLock<Option<T>>, stale: impl Fn(&T) -> bool) {
    if let Ok(mut cached) = cache.write() {
        if cached.as_ref().is_some_and(stale) {
            *cached = None;
        }
    }
}

fn remove_stale_indexes(storage: &Path, current: &Path) {
    let Ok(entries) = fs::read_dir(storage) else {
        return;
    };
    for entry in entries.flatten() {
        let path = entry.path();
        let stale = path != current
            && path.is_dir()
            && entry.file_name().to_string_lossy().starts_with("index-v");
        if !stale {
            continue;
        }
        match fs::remove_dir_all(&path) {
            Ok(()) => eprintln!("[aquilum:index] удалён индекс старой версии {}", path.display()),
            Err(error) => eprintln!("[aquilum:index] индекс старой версии не удалён {}: {error}", path.display()),
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

#[cfg(test)]
mod tests {
    use super::remove_stale_indexes;
    use std::fs;

    #[test]
    fn only_the_current_index_version_stays_on_disk() {
        let directory = tempfile::tempdir().unwrap();
        let storage = directory.path();
        for name in ["index-v6", "index-v8", "index-v9"] {
            fs::create_dir_all(storage.join(name).join("segment")).unwrap();
        }
        fs::write(storage.join("documents.sqlite3"), "").unwrap();

        remove_stale_indexes(storage, &storage.join("index-v9"));

        let mut left = fs::read_dir(storage)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect::<Vec<_>>();
        left.sort();
        assert_eq!(left, ["documents.sqlite3", "index-v9"]);
    }
}
