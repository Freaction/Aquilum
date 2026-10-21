use std::ops::Range;
use std::path::Path;

use aquilum_core::files::content::FileContent;
use aquilum_core::files::gate;
use aquilum_core::text::content::Content;
use aquilum_core::history::Source;
use masonry::app::RenderRoot;

use super::super::App;

pub type Change = Option<(Range<usize>, String)>;

impl App {
    pub(in crate::app) fn head(&self) -> String {
        crate::frontmatter::head(&crate::rope::Text(&self.text))
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

    pub(in crate::app) fn has_frontmatter(&self) -> bool {
        crate::frontmatter::bounds_of(&crate::rope::Text(&self.text)).is_some()
    }

    pub(in crate::app) fn fields(&self) -> Option<crate::frontmatter::Fields> {
        crate::frontmatter::parse(&self.head())
    }

    pub(in crate::app) fn shows(&self, path: &Path) -> bool {
        self.note.as_ref().is_some_and(|note| note.is(path))
    }

    pub(in crate::app) fn note_head(&self, path: &Path) -> Option<String> {
        if self.shows(path) {
            return Some(self.head());
        }
        aquilum_core::files::document::read_head(path).ok()
    }

    pub(in crate::app) fn anchor_offset(&self, path: &Path, anchor: &str) -> Option<usize> {
        use aquilum_core::search::pages::Pages;
        use aquilum_core::search::wiki::anchor::anchor_offset_in;
        if self.shows(path) {
            return anchor_offset_in(&mut Pages::of(&crate::rope::Text(&self.text)), anchor).ok().flatten();
        }
        anchor_offset_in(&mut Pages::open(path).ok()?, anchor).ok().flatten()
    }

    pub(in crate::app) fn change_note(&mut self, root: &mut RenderRoot, path: &Path, source: Source, change: impl Fn(&dyn Content) -> Change) -> bool {
        if self.shows(path) {
            let Some((range, insert)) = change(&crate::rope::Text(&self.text)) else { return false };
            if let Some(note) = &mut self.note {
                note.touch(range.clone(), insert.len());
                note.save_soon();
            }
            match &self.note_ids {
                Some(ids) if range.start >= self.hidden => {
                    let body = range.start - self.hidden..range.end - self.hidden;
                    ids.body.edit(root, |mut editor| crate::ui::editor::TextEditor::apply_external(&mut editor, body, &insert));
                    self.adopt_body(root);
                }
                _ => self.text.replace(range, &insert),
            }
            self.sync_body(root);
            return true;
        }
        let core = &self.workspace.core;
        let edited = gate::retrying(path, |hash| match change(&FileContent::open(path)?) {
            Some(change) => gate::edit(core, path, &[change], Some(hash), source, None).map(|_| true),
            None => Ok(false),
        });
        match edited {
            Ok(edited) => edited,
            Err(error) => {
                report!("{} не сохранён: {error:?}", path.display());
                false
            }
        }
    }
}
