use super::*;

impl TextEditor {
    pub(super) fn local(&self, offset: usize) -> (usize, usize, Option<&Layout<BrushIndex>>) {
        let i = self.doc.at_offset(offset);
        let p = self.doc.paragraph(i);
        (i, offset - p.start, p.layout.as_deref())
    }

    pub(super) fn laid_out(&self) -> bool {
        self.doc.measured()
    }

    pub(super) fn padding(&self) -> f64 {
        if self.single_line { 0.0 } else { size::CARET_WIDTH }
    }

    pub(super) fn inner_width(&self) -> f64 {
        (self.width - self.padding() * 2.0).max(0.0)
    }

    pub(super) fn shift(&self) -> Vec2 {
        let content = self.doc.paragraphs().first().and_then(|p| p.layout.as_ref()).map_or(0.0, |l| f64::from(l.width()));
        let center = if self.centered && self.single_line { ((self.inner_width() - content) / 2.0).max(0.0) } else { 0.0 };
        Vec2::new(self.padding() + center - self.scroll_x, 0.0)
    }

    pub(super) fn caret(&self) -> Rect {
        let compose = self.compose.as_ref().and_then(|(_, cursor)| *cursor).map_or(0, |(start, _)| start);
        self.caret_at(self.focus, compose, self.affinity)
    }

    pub(super) fn caret_at(&self, offset: usize, compose: usize, affinity: Affinity) -> Rect {
        let (i, local, layout) = self.local(offset);
        let Some(layout) = layout else {
            let p = self.doc.paragraph(i);
            let origin = p.origin();
            return Rect::new(origin.x, p.top, origin.x + size::CARET_WIDTH, p.top + p.height) + self.shift();
        };
        let cursor = Cursor::from_byte_index(layout, local + compose, affinity);
        let line = rect(cursor.geometry(layout, size::CARET_WIDTH as f32), 0.0);
        let middle = line.center().y as f32;
        let metrics = layout.lines().map(|l| *l.metrics()).find(|m| m.min_coord <= middle && middle < m.max_coord);
        let (top, bottom) = metrics.map_or((line.y0, line.y1), |m| (f64::from(m.min_coord), f64::from(m.max_coord)));
        let half = (bottom - top) / 2.0;
        let origin = self.doc.paragraph(i).origin();
        let center = (top + bottom) / 2.0 + origin.y;
        let line = line + Vec2::new(origin.x, 0.0);
        Rect::new(line.x0, center - half, line.x0 + size::CARET_WIDTH, center + half) + self.shift()
    }

    pub(super) fn reveal_caret(&mut self) {
        if !self.single_line || !self.laid_out() {
            return;
        }
        let inner = (self.inner_width() - size::CARET_WIDTH).max(0.0);
        let x = self.caret().x0 - self.shift().x;
        if x < self.scroll_x {
            self.scroll_x = x;
        } else if x > self.scroll_x + inner {
            self.scroll_x = x - inner;
        }
        let content = f64::from(self.doc.paragraph(0).layout().width());
        self.scroll_x = self.scroll_x.min((content - inner).max(0.0)).max(0.0);
    }

    pub(super) fn hit(&self, point: Point) -> (usize, Affinity) {
        let point = point - self.shift();
        if point.y < 0.0 && !self.single_line {
            return (0, Affinity::Downstream);
        }
        if point.y >= self.doc.height() && !self.single_line {
            return (self.doc.len(), Affinity::Downstream);
        }
        let i = self.doc.at_y(point.y.clamp(0.0, self.doc.height()));
        let p = self.doc.paragraph(i);
        if matches!(p.fold, Fold::Head | Fold::Member) {
            return (p.start, Affinity::Downstream);
        }
        let local = point - p.origin();
        let Some(layout) = p.layout.as_deref() else { return (p.start, Affinity::Downstream) };
        let cursor = Cursor::from_point(layout, local.x as f32, local.y as f32);
        (p.start + cursor.index(), cursor.affinity())
    }

    pub(super) fn step(&self, forward: bool, word: bool) -> (usize, Affinity) {
        let (i, local, layout) = self.local(self.focus);
        let p = self.doc.paragraph(i);
        let Some(layout) = layout else { return (if forward { p.end() } else { p.start }, Affinity::Downstream) };
        if forward && local >= p.len {
            return match self.doc.paragraphs().get(i + 1) {
                Some(next) => (next.start, Affinity::Downstream),
                None => (self.focus, self.affinity),
            };
        }
        if !forward && local == 0 {
            return match i.checked_sub(1).map(|j| self.doc.paragraph(j)) {
                Some(prev) => (prev.end(), Affinity::Downstream),
                None => (0, Affinity::Downstream),
            };
        }
        let cursor = Cursor::from_byte_index(layout, local, self.affinity);
        let moved = match (forward, word) {
            (true, false) => cursor.next_visual(layout),
            (false, false) => cursor.previous_visual(layout),
            (true, true) => cursor.next_visual_word(layout),
            (false, true) => cursor.previous_visual_word(layout),
        };
        (p.start + moved.index(), moved.affinity())
    }

    pub(super) fn vertical(&mut self, down: bool) -> (usize, Affinity) {
        if self.single_line {
            return if down { (self.doc.len(), Affinity::Downstream) } else { (0, Affinity::Downstream) };
        }
        let caret = self.caret();
        let x = *self.column.get_or_insert(caret.x0);
        let y = if down { caret.y1 + 1.0 } else { caret.y0 - 1.0 };
        self.hit(Point::new(x, y))
    }

    pub(super) fn line_edge(&self, end: bool) -> (usize, Affinity) {
        let (i, local, layout) = self.local(self.focus);
        let p = self.doc.paragraph(i);
        let Some(layout) = layout else { return (if end { p.end() } else { p.start }, Affinity::Downstream) };
        let selection = Selection::from_byte_index(layout, local, self.affinity);
        let moved = if end { selection.line_end(layout, false) } else { selection.line_start(layout, false) };
        let focus = moved.focus();
        (p.start + focus.index(), focus.affinity())
    }
}
