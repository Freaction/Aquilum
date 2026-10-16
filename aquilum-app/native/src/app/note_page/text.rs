use std::ops::Range;
use std::path::Path;
use std::time::Duration;

use aquilum_core::files::gate;
use aquilum_core::history::Source;
use masonry::app::RenderRoot;

use super::super::App;

pub type Change = Option<(Range<usize>, String)>;

impl App {
    pub(in crate::app) fn head(&self) -> String {
        crate::frontmatter::head(&self.text)
    }

    pub(in crate::app) fn body(&self) -> crop::Rope {
        let mut body = self.text.clone();
        body.delete(..self.hidden.min(body.byte_len()));
        body
    }

    pub(in crate::app) fn units_of(&self, byte: usize) -> usize {
        self.text.utf16_code_unit_of_byte(crate::rope::floor(&self.text, byte))
    }

    pub(in crate::app) fn byte_of(&self, units: usize) -> usize {
        self.text.byte_of_utf16_code_unit(units.min(self.text.utf16_len()))
    }

    pub(in crate::app) fn note_rope(&self, path: &Path) -> Option<crop::Rope> {
        if self.note.as_ref().is_some_and(|n| n.is(path)) {
            return Some(self.text.clone());
        }
        crate::note::read(path).ok().map(|loaded| loaded.text)
    }

    pub(in crate::app) fn note_text(&self, path: &Path) -> Option<String> {
        self.note_rope(path).map(|text| text.to_string())
    }

    pub(in crate::app) fn change_note(&mut self, root: &mut RenderRoot, path: &Path, source: Source, change: impl FnOnce(&crop::Rope) -> Change) -> bool {
        if self.note.as_ref().is_some_and(|n| n.is(path)) {
            let Some((range, insert)) = change(&self.text) else { return false };
            self.text.replace(range.clone(), &insert);
            if range.start >= self.hidden
                && let Some(ids) = &self.note_ids
            {
                let body = range.start - self.hidden..range.end - self.hidden;
                ids.body.edit(root, |mut editor| crate::ui::editor::TextEditor::apply_external(&mut editor, body, &insert));
            }
            if let Some(note) = &mut self.note {
                note.edit(Duration::ZERO);
            }
            self.sync_body(root);
            return true;
        }
        let Ok(loaded) = crate::note::read(path) else { return false };
        let Some((range, insert)) = change(&loaded.text) else { return false };
        let mut next = loaded.text;
        next.replace(range, &insert);
        match gate::write(&self.workspace.core, path, &next.to_string(), Some(&loaded.hash), source, None) {
            Ok(_) => true,
            Err(error) => {
                report!("{} не сохранён: {error:?}", path.display());
                false
            }
        }
    }
}
