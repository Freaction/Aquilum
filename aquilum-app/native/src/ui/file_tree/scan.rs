use std::path::Path;
use std::sync::Arc;
use std::sync::mpsc::{Receiver, channel};

use masonry::app::RenderRoot;

use super::FileTree;
use super::model::{self, Counts};

fn spawn<T: Send + 'static>(wake: Arc<dyn Fn() + Send + Sync>, work: impl FnOnce() -> T + Send + 'static) -> Receiver<T> {
    let (sender, receiver) = channel();
    std::thread::spawn(move || {
        if sender.send(work()).is_ok() {
            wake();
        }
    });
    receiver
}

fn ready<T>(slot: &mut Option<Receiver<T>>) -> Option<T> {
    let value = slot.as_ref()?.try_recv().ok()?;
    *slot = None;
    Some(value)
}

impl FileTree {
    pub fn reload(&mut self, root: &mut RenderRoot) {
        self.model.reload_all();
        self.count_notes();
        self.reindex();
        self.refresh(root);
    }

    pub(super) fn count_notes(&mut self) {
        let Some(root) = self.model.root().map(Path::to_path_buf) else { return };
        self.counting = Some(spawn(Arc::clone(&self.wake), move || Counts::scan(&root)));
    }

    pub(super) fn reindex(&mut self) {
        let Some(root) = self.model.root().filter(|_| self.model.filtering()).map(Path::to_path_buf) else { return };
        self.indexing = Some(spawn(Arc::clone(&self.wake), move || model::index(&root)));
    }

    pub fn poll(&mut self, root: &mut RenderRoot) {
        if let Some(index) = ready(&mut self.indexing) {
            self.model.set_index(index);
            self.refresh(root);
        }
        let Some(counts) = ready(&mut self.counting) else { return };
        self.model.set_counts(counts);
        self.refresh(root);
    }
}
