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
use std::time::{Duration, Instant};

const EVENT_QUEUE_CAPACITY: usize = 2_048;
const EVENT_BATCH_WINDOW: Duration = Duration::from_millis(80);

pub struct WorkerHandle {
    sender: mpsc::SyncSender<WorkerMessage>,
    dirty: Arc<AtomicBool>,
    cancelled: Arc<AtomicBool>,
    thread: Option<JoinHandle<()>>,
}

enum WorkerMessage {
    Paths(Vec<PathBuf>),
    Rescan(Vec<PathBuf>),
}

struct WorkerContext {
    root: PathBuf,
    index: Arc<SearchIndex>,
    metadata_path: PathBuf,
    dirty: Arc<AtomicBool>,
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
        let (sender, receiver) = mpsc::sync_channel(EVENT_QUEUE_CAPACITY);
        let dirty = Arc::new(AtomicBool::new(false));
        let cancelled = Arc::new(AtomicBool::new(false));
        let context = WorkerContext {
            root,
            index,
            metadata_path,
            dirty: Arc::clone(&dirty),
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
        Ok(Self {
            sender,
            dirty,
            cancelled,
            thread: Some(thread),
        })
    }

    pub fn queue_paths(&self, paths: Vec<PathBuf>) {
        enqueue(&self.sender, &self.dirty, WorkerMessage::Paths(paths));
    }

    pub fn queue_rescan(&self, paths: Vec<PathBuf>) {
        enqueue(&self.sender, &self.dirty, WorkerMessage::Rescan(paths));
    }

    pub fn stop(mut self) {
        self.cancelled.store(true, Ordering::Relaxed);
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
    synchronizer.full_scan(&context.cancelled)?;
    notify_index(context);

    while !context.cancelled.load(Ordering::Relaxed) {
        if context.dirty.swap(false, Ordering::Relaxed) {
            synchronizer.full_scan(&context.cancelled)?;
            notify_index(context);
            continue;
        }
        match receiver.recv_timeout(Duration::from_millis(300)) {
            Ok(message) => {
                let (paths, rescan) = collect_batch(message, &receiver);
                if rescan {
                    synchronizer.full_scan(&context.cancelled)?;
                    notify_index(context);
                } else if synchronizer.apply_paths(paths)? {
                    notify_index(context);
                }
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => return Ok(()),
        }
    }
    Ok(())
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
    initial: WorkerMessage,
    receiver: &mpsc::Receiver<WorkerMessage>,
) -> (HashSet<PathBuf>, bool) {
    let deadline = Instant::now() + EVENT_BATCH_WINDOW;
    let (initial, mut rescan) = match initial {
        WorkerMessage::Paths(paths) => (paths, false),
        WorkerMessage::Rescan(paths) => (paths, true),
    };
    let mut paths = initial.into_iter().collect::<HashSet<_>>();
    loop {
        let Some(remaining) = deadline.checked_duration_since(Instant::now()) else {
            return (paths, rescan);
        };
        match receiver.recv_timeout(remaining) {
            Ok(WorkerMessage::Paths(next)) => paths.extend(next),
            Ok(WorkerMessage::Rescan(next)) => {
                paths.extend(next);
                rescan = true;
            }
            Err(_) => return (paths, rescan),
        }
    }
}

fn enqueue(sender: &mpsc::SyncSender<WorkerMessage>, dirty: &AtomicBool, message: WorkerMessage) {
    if sender.try_send(message).is_err() {
        dirty.store(true, Ordering::Relaxed);
    }
}

fn panic_message(payload: &(dyn std::any::Any + Send)) -> String {
    payload
        .downcast_ref::<&str>()
        .map(|text| (*text).to_owned())
        .or_else(|| payload.downcast_ref::<String>().cloned())
        .unwrap_or_else(|| "без сообщения".to_owned())
}
