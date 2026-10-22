use super::error::SearchError;
use super::index::SearchIndex;
use super::metadata;
use super::models::IndexRevision;
use super::paths::cache_key;
use super::progress::IndexProgress;
use super::service::{clear_if, OpenIndex, SearchService};
use super::worker::WorkerHandle;
use super::IndexNotifier;
use crate::files::remove_matching;
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, PoisonError};

fn index_name() -> String {
    format!("index-v{}", crate::search::ANALYZER_VERSION)
}

impl SearchService {
    pub fn open_index(&self, root: PathBuf, visible: bool) -> Result<OpenIndex, SearchError> {
        fs::create_dir_all(&self.base_directory)?;
        let storage = self.base_directory.join(cache_key(&root));
        let generation = self.next_generation.fetch_add(1, Ordering::Relaxed) + 1;
        let index = Arc::new(SearchIndex::open(&storage.join(index_name()))?);
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

    pub fn close(&self) {
        *self.prepare_lock.lock().unwrap_or_else(PoisonError::into_inner) = true;
        let active = self.active.write().ok().and_then(|mut slot| slot.take());
        let guest = self.guest.write().ok().and_then(|mut slot| slot.take());
        drop((active, guest));
        self.forget_analysis();
    }

    pub fn prune(&self, keep: &HashSet<String>) {
        let Ok(_opening) = self.opening() else { return };
        let guest = self.guest.read().ok().and_then(|guest| guest.as_ref().map(|guest| guest.index.root.clone()));
        let open = [self.active_root(), guest].iter().flatten().map(|root| cache_key(root)).collect::<Vec<_>>();
        remove_matching(&self.base_directory, |name| !keep.contains(name) && !open.iter().any(|key| key == name));
        let current = index_name();
        for entry in fs::read_dir(&self.base_directory).into_iter().flatten().flatten() {
            remove_matching(&entry.path(), |name| name.starts_with("index-v") && name != current);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::search::paths::canonical_path;
    use std::path::Path;

    fn listing(dir: &Path) -> Vec<String> {
        let mut names = fs::read_dir(dir).unwrap().map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned()).collect::<Vec<_>>();
        names.sort();
        names
    }

    fn service(directory: &tempfile::TempDir) -> (SearchService, String) {
        let workspace = canonical_path(directory.path()).join("vault");
        fs::create_dir_all(&workspace).unwrap();
        fs::write(workspace.join("Заметка.md"), "# Заметка\n\nтекст").unwrap();
        (SearchService::new(directory.path().join("search"), Arc::new(|_| {})), workspace.to_string_lossy().into_owned())
    }

    #[test]
    fn only_known_and_open_caches_keep_their_current_index_on_disk() {
        let directory = tempfile::tempdir().unwrap();
        let (service, workspace) = service(&directory);
        service.prepare(&workspace).unwrap();
        let base = directory.path().join("search");
        let (known, forgotten, open) = ("a".repeat(64), "b".repeat(64), cache_key(Path::new(&workspace)));
        for name in ["index-v6".to_owned(), "index-v8".to_owned(), index_name()] {
            fs::create_dir_all(base.join(&known).join(name).join("segment")).unwrap();
        }
        fs::write(base.join(&known).join("documents.sqlite3"), "").unwrap();
        fs::create_dir_all(base.join(&forgotten).join(index_name())).unwrap();

        service.prune(&HashSet::from([known.clone()]));

        let mut expected = vec![known.clone(), open.clone()];
        expected.sort();
        assert_eq!(listing(&base), expected);
        assert_eq!(listing(&base.join(&known)), ["documents.sqlite3".to_owned(), index_name()]);
        assert!(listing(&base.join(&open)).contains(&index_name()), "открытый индекс не тронут");
        service.close();
    }

    #[test]
    fn a_closed_service_opens_nothing_and_leaves_no_file_open() {
        let directory = tempfile::tempdir().unwrap();
        let (service, workspace) = service(&directory);
        service.prepare(&workspace).unwrap();

        service.close();

        assert!(matches!(service.prepare(&workspace), Err(SearchError::Unavailable { .. })));
        assert!(matches!(service.open_guest(&workspace), Err(SearchError::Unavailable { .. })));
        directory.close().expect("индекс закрыт, каталог удаляется целиком");
    }
}
