use std::path::Path;
use std::sync::Arc;

use masonry::app::RenderRoot;

use super::FileTree;
use super::model::Counts;

impl FileTree {
    pub fn reload(&mut self, root: &mut RenderRoot) {
        self.model.reload_all();
        self.count_notes(Arc::clone(&self.wake));
        self.refresh(root);
    }

    pub(super) fn count_notes(&mut self, wake: Arc<dyn Fn() + Send + Sync>) {
        let Some(root) = self.model.root().map(Path::to_path_buf) else { return };
        let (sender, receiver) = std::sync::mpsc::channel();
        std::thread::spawn(move || {
            if sender.send(Counts::scan(&root)).is_ok() {
                wake();
            }
        });
        self.counting = Some(receiver);
    }

    pub fn poll(&mut self, root: &mut RenderRoot) {
        let Some(counts) = self.counting.as_ref().and_then(|receiver| receiver.try_recv().ok()) else { return };
        self.counting = None;
        self.model.set_counts(counts);
        self.refresh(root);
    }
}
