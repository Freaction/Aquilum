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
        self.view = Some((self.hidden + offset, px));
        self.view_due = Some(Instant::now() + VIEW_SAVE_DELAY);
    }

    pub fn restore_view(&mut self, root: &mut RenderRoot) -> bool {
        let Some((anchor, px)) = self.restore else { return false };
        let Some((viewport, editor, offset)) = self.scroll_geometry(root) else { return false };
        let Some(ids) = &self.note_ids else { return false };
        let byte = anchor.saturating_sub(self.hidden);
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
        let units = self.view.map(|(anchor, px)| (self.units_of(anchor), px));
        let (Some(session), Some(note), Some((units, px))) = (self.session.as_mut(), self.note.as_ref(), units) else { return };
        if let Some(document) = session.document(&self.workspace.core, &note.path) {
            let _zone = aq_trace::zone("место в заметке: запись");
            session.save_view(&self.workspace.core, document, units as i64, px);
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
