use super::error::SearchError;
use super::index::SearchIndex;
use super::progress::IndexProgress;
use super::sync::IndexSynchronizer;
use super::models::IndexRevision;
use super::IndexNotifier;
use std::collections::HashSet;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc};
use std::thread::JoinHandle;

pub struct WorkerHandle {
    sender: mpsc::Sender<WorkerMessage>,
    cancelled: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

enum WorkerMessage {
    Paths(Vec<PathBuf>),
    Rescan,
}

struct WorkerContext {
    root: PathBuf,
    index: Arc<SearchIndex>,
    metadata_path: PathBuf,
    cancelled: Arc<AtomicBool>,
    progress: Arc<IndexProgress>,
    generation: u64,
    index_notifier: IndexNotifier,
}

impl WorkerHandle {
    pub fn spawn(
        root: PathBuf,
        index: Arc<SearchIndex>,
        metadata_path: PathBuf,
        progress: Arc<IndexProgress>,
        generation: u64,
        index_notifier: IndexNotifier,
    ) -> Result<Self, SearchError> {
        let (sender, receiver) = mpsc::channel();
        let cancelled = Arc::new(AtomicBool::new(false));
        let context = WorkerContext {
            root,
            index,
            metadata_path,
            cancelled: Arc::clone(&cancelled),
            progress: Arc::clone(&progress),
            generation,
            index_notifier,
        };
        let thread = std::thread::Builder::new()
            .name("aquilum-search-index".to_owned())
            .spawn(move || {
                let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| run(&context, receiver)));
                let failure = match outcome {
                    Ok(Ok(())) => None,
                    Ok(Err(error)) => Some(error),
                    Err(panic) => Some(SearchError::Task {
                        message: format!("поток индексации упал: {}", panic_message(panic.as_ref())),
                    }),
                };
                if let Some(error) = failure {
                    progress.failed(&error);
                }
            })
            .map_err(|error| SearchError::Task {
                message: error.to_string(),
            })?;
        Ok(Self { sender, cancelled, thread: Some(thread) })
    }

    pub fn queue_paths(&self, paths: Vec<PathBuf>) {
        let _ = self.sender.send(WorkerMessage::Paths(paths));
    }

    pub fn queue_rescan(&self) {
        let _ = self.sender.send(WorkerMessage::Rescan);
    }

}

impl Drop for WorkerHandle {
    fn drop(&mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
        let _ = self.sender.send(WorkerMessage::Rescan);
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

fn run(
    context: &WorkerContext,
    receiver: mpsc::Receiver<WorkerMessage>,
) -> Result<(), SearchError> {
    let mut synchronizer = IndexSynchronizer::new(
        context.root.clone(),
        Arc::clone(&context.index),
        &context.metadata_path,
        Arc::clone(&context.progress),
    )?;
    let stopped = || context.cancelled.load(Ordering::Relaxed);
    let mut message = WorkerMessage::Rescan;
    loop {
        let (paths, rescan) = collect_batch(message, &receiver);
        let outcome = if rescan { synchronizer.full_scan(&context.cancelled).map(|()| true) } else { synchronizer.apply_paths(paths) };
        let retry = match outcome {
            Ok(changed) => {
                if changed && !stopped() {
                    notify_index(context);
                }
                false
            }
            Err(error) => {
                context.progress.failed(&error);
                true
            }
        };
        if stopped() {
            return Ok(());
        }
        message = match receiver.recv() {
            Ok(_) if retry && !stopped() => WorkerMessage::Rescan,
            Ok(next) if !stopped() => next,
            _ => return Ok(()),
        };
    }
}

fn notify_index(context: &WorkerContext) {
    let revision = context.progress.bump_revision();
    (context.index_notifier)(IndexRevision {
        workspace_path: context.root.to_string_lossy().into_owned(),
        generation: context.generation,
        revision,
    });
}

fn collect_batch(
    first: WorkerMessage,
    receiver: &mpsc::Receiver<WorkerMessage>,
) -> (HashSet<PathBuf>, bool) {
    let (mut paths, mut rescan) = (HashSet::new(), false);
    for message in std::iter::once(first).chain(receiver.try_iter()) {
        match message {
            WorkerMessage::Paths(next) => paths.extend(next),
            WorkerMessage::Rescan => rescan = true,
        }
    }
    (paths, rescan)
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "без сообщения".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{Duration, Instant};

    #[test]
    fn an_idle_worker_stops_at_once() {
        let directory = tempfile::tempdir().unwrap();
        let root = crate::search::paths::canonical_path(directory.path()).join("vault");
        std::fs::create_dir_all(&root).unwrap();
        let index = Arc::new(SearchIndex::open(&directory.path().join("index")).unwrap());
        let (scanned, done) = mpsc::channel();
        let notifier: IndexNotifier = Arc::new(move |_| {
            let _ = scanned.send(());
        });
        let worker = WorkerHandle::spawn(root, index, directory.path().join("documents.sqlite3"), Arc::new(IndexProgress::new(0, 1)), 1, notifier).unwrap();
        done.recv_timeout(Duration::from_secs(20)).expect("первый проход завершился");

        let started = Instant::now();
        drop(worker);

        assert!(started.elapsed() < Duration::from_millis(100), "остановка ждала {:?}", started.elapsed());
    }
}
