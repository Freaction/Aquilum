use std::path::{Path, PathBuf};

use aquilum_core::search::paths::same_path;
use masonry::app::RenderRoot;

use super::super::App;
use crate::file_ops;

impl App {
    pub(in crate::app) fn follow_move(&mut self, root: &mut RenderRoot, from: &Path, to: &Path) {
        self.interface.move_metadata(from, to);
        if let Some(note) = self.note.as_mut().filter(|n| n.is(from)) {
            note.path = to.to_path_buf();
            self.workspace.core.active_note.set(Some(note.path.clone()));
            self.tree.set_active(root, Some(note.path.clone()));
            self.refresh_note_names(root);
        }
        self.relocate_tabs(root, from, to);
    }

    pub(in crate::app) fn rename_file(&mut self, root: &mut RenderRoot, from: PathBuf, to: PathBuf) {
        self.commit_note();
        self.follow_move(root, &from, &to);
        let (failed, wake) = (self.failed_renames.0.clone(), std::sync::Arc::clone(&self.workspace.wake));
        file_ops::rename(&self.workspace.core, from.clone(), to.clone(), move || {
            let _ = failed.send((from, to));
            wake();
        });
    }

    pub(in crate::app) fn undo_failed_renames(&mut self, root: &mut RenderRoot) {
        while let Ok((from, to)) = self.failed_renames.1.try_recv() {
            self.follow_move(root, &to, &from);
        }
    }

    pub(in crate::app) fn relocate_tabs(&mut self, root: &mut RenderRoot, from: &Path, to: &Path) {
        if !self.tabs.rename(from, to) {
            return;
        }
        if let Some(session) = &self.session {
            for tab in self.tabs.tabs().iter().filter(|t| t.path.as_deref() == Some(to)) {
                session.renamed(&self.workspace.core, tab);
            }
        }
        self.refresh_tabs(root);
    }

    pub(in crate::app) fn close_missing_tabs(&mut self, root: &mut RenderRoot, removed: &[PathBuf]) -> bool {
        let inside = |path: &Path| path.ancestors().any(|ancestor| removed.iter().any(|gone| same_path(ancestor, gone)));
        let gone: Vec<_> = self.tabs.tabs().iter().filter(|tab| tab.path.as_ref().is_some_and(|p| !p.exists() && inside(p))).map(|tab| tab.id).collect();
        for id in &gone {
            self.tabs.close(*id);
        }
        if !gone.is_empty() {
            self.show_active(root);
        }
        !gone.is_empty()
    }
}
