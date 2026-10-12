use std::path::Path;
use std::time::Duration;

use aquilum_core::files::document::{read_file_snapshot_impl, read_text};
use aquilum_core::files::gate;
use aquilum_core::history::Source;
use masonry::app::RenderRoot;

use super::App;

impl App {
    pub(in crate::app) fn note_text(&self, path: &Path) -> Option<String> {
        if self.note.as_ref().is_some_and(|n| n.is(path)) {
            return Some(self.editor_text.clone());
        }
        read_text(path).ok()
    }

    pub(in crate::app) fn edit_note_text(&mut self, root: &mut RenderRoot, path: &Path, source: Source, edit: impl FnOnce(&str) -> Option<String>) -> bool {
        if self.note.as_ref().is_some_and(|n| n.is(path)) {
            let Some(next) = edit(&self.editor_text).filter(|next| *next != self.editor_text) else { return false };
            self.editor_text = next;
            if let Some(note) = &mut self.note {
                note.edit(Duration::ZERO);
                let _ = note.save(&self.workspace.core, &self.editor_text);
            }
            self.sync_body(root);
            return true;
        }
        let Ok(snapshot) = read_file_snapshot_impl(path) else { return false };
        let Some(next) = edit(&snapshot.content).filter(|next| *next != snapshot.content) else { return false };
        match gate::write(&self.workspace.core, path, &next, Some(&snapshot.hash), source, None) {
            Ok(_) => true,
            Err(error) => {
                report!("{} не сохранён: {error:?}", path.display());
                false
            }
        }
    }
}
