use super::*;
use crate::ui::editor::embed::View;

impl TextEditor {
    pub(super) fn index_at(&self, point: Point) -> Option<usize> {
        (self.doc.markdown.is_some() && self.laid_out()).then(|| self.doc.at_y(point.y.max(0.0)))
    }

    pub(super) fn embed_local(&self, top: f64, point: Point) -> Point {
        Point::new(point.x - self.shift().x, point.y - top)
    }

    pub(super) fn embed_local_at(&self, start: usize, point: Point) -> Point {
        self.embed_local(self.doc.top(self.doc.at_offset(start)), point)
    }

    pub(super) fn head_embed_at(&self, point: Point) -> Option<(Paragraph<'_>, &View, Point)> {
        let i = self.index_at(point)?;
        let p = self.doc.paragraph(i);
        let embed = p.embed.filter(|_| matches!(p.fold, Fold::Head))?;
        Some((p, &embed.view, self.embed_local(self.doc.top(i), point)))
    }
}
