pub mod batch;
pub mod classify;

pub use batch::{WatchBatch, WatchScope};

use super::attachments::{forget_attachments, forget_attachments_after};
use batch::coalesce;
use classify::classify;
use notify::{RecursiveMode, Watcher};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{mpsc, Arc, Mutex};
use std::thread::JoinHandle;

pub type WatchSink = Arc<dyn Fn(WatchBatch) + Send + Sync>;

pub struct WorkspaceWatcher {
    running: Mutex<Option<Running>>,
    closed: AtomicBool,
    sink: WatchSink,
}

struct Running {
    root: PathBuf,
    stopping: Arc<AtomicBool>,
    wake: mpsc::Sender<WatchBatch>,
    thread: Option<JoinHandle<()>>,
}

impl Running {
    fn stop(mut self) {
        self.stopping.store(true, Ordering::Relaxed);
        let _ = self.wake.send(WatchBatch::rescan());
        if let Some(thread) = self.thread.take() {
            let _ = thread.join();
        }
    }
}

impl WorkspaceWatcher {
    pub fn new(sink: WatchSink) -> Self {
        Self {
            running: Mutex::new(None),
            closed: AtomicBool::new(false),
            sink,
        }
    }

    pub fn watch(&self, root: &Path) -> Result<(), String> {
        let mut running = self.running.lock().unwrap_or_else(|error| error.into_inner());
        if self.closed.load(Ordering::Relaxed) {
            return Err("наблюдатель закрыт".to_owned());
        }
        if running.as_ref().is_some_and(|active| active.root == root) {
            return Ok(());
        }
        if let Some(previous) = running.take() {
            previous.stop();
        }

        let (sender, receiver) = mpsc::channel::<WatchBatch>();
        let wake = sender.clone();
        forget_attachments();
        let watched = root.to_path_buf();
        let mut watcher = notify::recommended_watcher(move |result| {
            forget_attachments_after(&result);
            if let Some(batch) = classify(result, &watched) {
                let _ = sender.send(batch);
            }
        })
        .map_err(|error| format!("не удалось создать наблюдатель: {error}"))?;
        watcher
            .watch(root, RecursiveMode::Recursive)
            .map_err(|error| format!("не удалось наблюдать за {}: {error}", root.display()))?;

        let stopping = Arc::new(AtomicBool::new(false));
        let thread_stopping = Arc::clone(&stopping);
        let sink = Arc::clone(&self.sink);
        let thread = std::thread::Builder::new()
            .name("aquilum-fs-watch".to_owned())
            .spawn(move || deliver(watcher, &receiver, &thread_stopping, sink.as_ref()))
            .map_err(|error| format!("не удалось запустить поток наблюдателя: {error}"))?;

        *running = Some(Running {
            root: root.to_path_buf(),
            stopping,
            wake,
            thread: Some(thread),
        });
        Ok(())
    }

    pub fn close(&self) {
        let mut running = self.running.lock().unwrap_or_else(|error| error.into_inner());
        self.closed.store(true, Ordering::Relaxed);
        if let Some(previous) = running.take() {
            previous.stop();
        }
    }
}

fn deliver(
    _watcher: impl Watcher,
    receiver: &mpsc::Receiver<WatchBatch>,
    stopping: &AtomicBool,
    sink: &(dyn Fn(WatchBatch) + Send + Sync),
) {
    while let Ok(first) = receiver.recv() {
        if stopping.load(Ordering::Relaxed) {
            return;
        }
        let batch = coalesce(first, receiver);
        if stopping.load(Ordering::Relaxed) {
            return;
        }
        sink(batch);
    }
}

#[cfg(test)]
mod tests;
