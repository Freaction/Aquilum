use super::*;
use crate::ui::editor::embed::View;

impl TextEditor {
    pub(super) fn paragraph_at(&self, point: Point) -> Option<&Paragraph> {
        (self.doc.markdown.is_some() && self.laid_out()).then(|| self.doc.paragraph(self.doc.at_y(point.y.max(0.0))))
    }

    pub(super) fn embed_local(&self, top: f64, point: Point) -> Point {
        Point::new(point.x - self.shift().x, point.y - top)
    }

    pub(super) fn embed_local_at(&self, start: usize, point: Point) -> Point {
        self.embed_local(self.doc.paragraph(self.doc.at_offset(start)).top, point)
    }

    pub(super) fn head_embed_at(&self, point: Point) -> Option<(&Paragraph, &View, Point)> {
        let p = self.paragraph_at(point)?;
        let embed = p.embed.as_deref().filter(|_| matches!(p.fold, Fold::Head))?;
        Some((p, &embed.view, self.embed_local(p.top, point)))
    }
}
