use std::path::Path;
use std::sync::mpsc::{Receiver, channel};

use masonry::app::RenderRoot;

use super::FileTree;
use super::model::{self, Counts};
use crate::app::jobs;

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
        let (sender, receiver) = channel();
        jobs::deliver("дерево: счёт заметок", sender, &self.wake, move || Some(Counts::scan(&root)));
        self.counting = Some(receiver);
    }

    pub(super) fn reindex(&mut self) {
        let Some(root) = self.model.root().filter(|_| self.model.filtering()).map(Path::to_path_buf) else { return };
        let (sender, receiver) = channel();
        jobs::deliver("дерево: список для фильтра", sender, &self.wake, move || Some(model::index(&root)));
        self.indexing = Some(receiver);
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
