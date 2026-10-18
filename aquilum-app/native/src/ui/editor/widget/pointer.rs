use super::*;
use crate::ui::editor::embed::View;

impl TextEditor {
    pub(super) fn index_at(&self, point: Point) -> Option<usize> {
        (self.doc.markdown.is_some() && self.laid_out()).then(|| self.doc.at_y(point.y.max(0.0)))
    }

    pub(super) fn embed_x(&self, i: usize) -> f64 {
        self.shift().x + self.doc.paragraph(i).embed.map_or(0.0, |embed| embed.x)
    }

    pub(super) fn embed_local(&self, i: usize, point: Point) -> Point {
        Point::new(point.x - self.embed_x(i), point.y - self.doc.top(i))
    }

    pub(super) fn embed_local_at(&self, start: usize, point: Point) -> Point {
        self.embed_local(self.doc.at_offset(start), point)
    }

    pub(super) fn head_embed_at(&self, point: Point) -> Option<(Paragraph<'_>, &View, Point)> {
        let i = self.index_at(point)?;
        let p = self.doc.paragraph(i);
        let embed = p.embed.filter(|_| matches!(p.fold, Fold::Head))?;
        Some((p, &embed.view, self.embed_local(i, point)))
    }
}
