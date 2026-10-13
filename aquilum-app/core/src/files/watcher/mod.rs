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
            sink,
        }
    }

    pub fn watch(&self, root: &Path) -> Result<(), String> {
        let mut running = self.running.lock().unwrap_or_else(|error| error.into_inner());
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

    pub fn stop(&self) {
        let mut running = self.running.lock().unwrap_or_else(|error| error.into_inner());
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
mod tests {
    use super::*;
    use std::sync::mpsc::RecvTimeoutError;
    use std::time::Duration;

    fn collector() -> (WatchSink, mpsc::Receiver<WatchBatch>) {
        let (sender, receiver) = mpsc::channel();
        let sink: WatchSink = Arc::new(move |batch| {
            let _ = sender.send(batch);
        });
        (sink, receiver)
    }

    #[test]
    fn refuses_to_start_on_a_missing_folder_instead_of_dying_quietly() {
        let (sink, _receiver) = collector();
        let watcher = WorkspaceWatcher::new(sink);

        let error = watcher
            .watch(Path::new("D:/такой/папки/нет/и/не/будет"))
            .unwrap_err();

        assert!(error.contains("не удалось наблюдать"));
    }

    #[test]
    fn delivers_a_real_file_change_to_the_sink() {
        let directory = tempfile::tempdir().unwrap();
        let (sink, receiver) = collector();
        let watcher = WorkspaceWatcher::new(sink);
        watcher.watch(directory.path()).unwrap();

        std::fs::write(directory.path().join("Заметка.md"), "текст").unwrap();

        let batch = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("вотчер обязан доставить изменение файла");
        assert!(batch
            .paths
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "Заметка.md")));
        watcher.stop();
    }

    #[test]
    fn retargeting_moves_the_watch_to_the_new_workspace() {
        let first = tempfile::tempdir().unwrap();
        let second = tempfile::tempdir().unwrap();
        let (sink, receiver) = collector();
        let watcher = WorkspaceWatcher::new(sink);
        watcher.watch(first.path()).unwrap();
        watcher.watch(second.path()).unwrap();

        std::fs::write(second.path().join("Вторая.md"), "текст").unwrap();
        let batch = receiver
            .recv_timeout(Duration::from_secs(10))
            .expect("новая база обязана наблюдаться");
        assert!(batch
            .paths
            .iter()
            .any(|path| path.file_name().is_some_and(|name| name == "Вторая.md")));

        std::fs::write(first.path().join("Первая.md"), "текст").unwrap();
        assert!(
            matches!(
                receiver.recv_timeout(Duration::from_millis(800)),
                Err(RecvTimeoutError::Timeout)
            ),
            "старая база больше не наблюдается"
        );
        watcher.stop();
    }

    #[test]
    fn stop_does_not_wait_for_a_poll_interval() {
        let directory = tempfile::tempdir().unwrap();
        let (sink, _receiver) = collector();
        let watcher = WorkspaceWatcher::new(sink);
        watcher.watch(directory.path()).unwrap();
        std::thread::sleep(Duration::from_millis(50));

        let started = std::time::Instant::now();
        watcher.stop();

        assert!(started.elapsed() < Duration::from_millis(100), "остановка ждала {:?}", started.elapsed());
    }

    #[test]
    fn stops_delivering_after_stop() {
        let directory = tempfile::tempdir().unwrap();
        let (sink, receiver) = collector();
        let watcher = WorkspaceWatcher::new(sink);
        watcher.watch(directory.path()).unwrap();
        watcher.stop();

        std::fs::write(directory.path().join("После.md"), "текст").unwrap();

        assert!(matches!(
            receiver.recv_timeout(Duration::from_millis(800)),
            Err(RecvTimeoutError::Timeout)
        ));
    }
}
