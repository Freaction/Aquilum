use std::time::{Duration, Instant};

use masonry::app::RenderRoot;

use super::super::App;

pub const VIEW_SAVE_DELAY: Duration = Duration::from_millis(500);

impl App {
    pub(in crate::app) fn scroll_geometry(&self, root: &RenderRoot) -> Option<(f64, f64, f64)> {
        let ids = self.note_ids.as_ref()?;
        let scroll = root.get_widget(ids.scroll.id())?;
        let body = root.get_widget(ids.body.id())?;
        let top = |t: masonry::kurbo::Affine| (t * masonry::kurbo::Point::ZERO).y;
        let offset = scroll.downcast::<crate::ui::scroll::ScrollArea<masonry::widgets::Flex>>()?.inner().offset();
        Some((top(scroll.ctx().window_transform()), top(body.ctx().window_transform()), offset))
    }

    pub(in crate::app) fn capture_view(&mut self, root: &RenderRoot) {
        let Some((viewport, editor, _)) = self.scroll_geometry(root) else { return };
        let Some(ids) = &self.note_ids else { return };
        let Some(body) = root.get_widget(ids.body.id()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()) else { return };
        let Some((offset, px)) = body.inner().anchor_at(viewport - editor) else { return };
        let anchor = self.editor_text[..(self.hidden + offset).min(self.editor_text.len())].encode_utf16().count() as i64;
        self.view = Some((anchor, px));
        self.view_due = Some(Instant::now() + VIEW_SAVE_DELAY);
    }

    pub fn restore_view(&mut self, root: &mut RenderRoot) -> bool {
        let Some((anchor, px)) = self.restore else { return false };
        let Some((viewport, editor, offset)) = self.scroll_geometry(root) else { return false };
        let Some(ids) = &self.note_ids else { return false };
        let byte = aquilum_core::documents::merge::utf16_byte(&self.editor_text, usize::try_from(anchor).unwrap_or(0)).saturating_sub(self.hidden);
        let Some(top) = root.get_widget(ids.body.id()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).and_then(|b| b.inner().top_of(byte)) else {
            return false;
        };
        let target = editor - viewport + offset + top + px;
        let scroll = ids.scroll;
        scroll.edit(root, |mut area| crate::ui::scroll::ScrollArea::set_offset(&mut area, target));
        self.restore = None;
        true
    }

    pub(in crate::app) fn flush_view(&mut self) {
        self.view_due = None;
        let (Some(session), Some(note), Some((anchor, px))) = (self.session.as_mut(), self.note.as_ref(), self.view) else { return };
        if let Some(document) = session.document(&self.workspace.core, &note.path) {
            session.save_view(&self.workspace.core, document, anchor, px);
        }
    }

    /// Прокручивает заметку так, чтобы место `byte` текста (без метаданных) оказалось посередине.
    pub(crate) fn reveal(&mut self, root: &mut RenderRoot, byte: usize) {
        let Some((viewport, editor, offset)) = self.scroll_geometry(root) else { return };
        let Some(ids) = &self.note_ids else { return };
        let Some(top) = root.get_widget(ids.body.id()).and_then(|w| w.downcast::<crate::ui::editor::TextEditor>()).and_then(|b| b.inner().top_of(byte)) else { return };
        let height = root.get_widget(ids.scroll.id()).map_or(0.0, |w| w.ctx().bounding_box().height());
        let target = editor - viewport + offset + top - height / 2.0;
        let scroll = ids.scroll;
        scroll.edit(root, |mut area| crate::ui::scroll::ScrollArea::set_offset(&mut area, target));
    }
}

