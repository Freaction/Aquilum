use super::*;
use crate::ui::editor::caret;

impl TextEditor {
    pub(super) fn local(&self, offset: usize) -> (usize, usize, Option<&Layout<BrushIndex>>) {
        let i = self.doc.at_offset(offset);
        let p = self.doc.paragraph(i);
        (i, offset.clamp(p.start, p.end()) - p.start, p.laid())
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
        let content = self.doc.paragraph(0).laid().map_or(0.0, |l| f64::from(l.width()));
        let center = if self.centered && self.single_line { ((self.inner_width() - content) / 2.0).max(0.0) } else { 0.0 };
        Vec2::new(self.padding() + center - self.scroll_x, 0.0)
    }

    pub(super) fn caret(&self) -> Rect {
        if let Some((caret, _)) = self.table_caret() {
            return caret;
        }
        let compose = self.compose.as_ref().and_then(|(_, cursor)| *cursor).map_or(0, |(start, _)| start);
        self.caret_at(self.focus, compose, self.affinity)
    }

    pub(super) fn caret_at(&self, offset: usize, compose: usize, affinity: Affinity) -> Rect {
        let (start, end) = self.doc.loaded_range();
        if offset < start || offset > end {
            let y = self.doc.y_of(offset);
            return Rect::new(0.0, y, size::CARET_WIDTH, y + self.doc.line_height()) + self.shift();
        }
        let (i, local, layout) = self.local(offset);
        let Some(layout) = layout else {
            let origin = self.doc.origin(i);
            return Rect::new(origin.x, self.doc.top(i), origin.x + size::CARET_WIDTH, self.doc.bottom(i)) + self.shift();
        };
        let cursor = Cursor::from_byte_index(layout, local + compose, affinity);
        let line = rect(cursor.geometry(layout, size::CARET_WIDTH as f32), 0.0);
        let (top, bottom) = markdown::line_at(layout, line.center().y).map_or((line.y0, line.y1), |m| (f64::from(m.block_min_coord), f64::from(m.block_max_coord)));
        let half = (bottom - top) / 2.0;
        let origin = self.doc.origin(i);
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
            let head = (0..=i).rev().find(|&j| self.doc.paragraph(j).fold == Fold::Head).unwrap_or(i);
            let below = point.y > (self.doc.top(head) + self.doc.bottom(head)) / 2.0;
            return (if below { (self.doc.folded_end(head) + 1).min(self.doc.len()) } else { self.doc.paragraph(head).start }, Affinity::Downstream);
        }
        let Some(layout) = p.laid() else { return (p.start, Affinity::Downstream) };
        let (local, affinity) = caret::hit(layout, self.doc.slice(p.range()), point - self.doc.origin(i));
        (p.start + local, affinity)
    }

    pub(super) fn step(&self, forward: bool, word: bool) -> (usize, Affinity) {
        let (i, local, layout) = self.local(self.focus);
        let p = self.doc.paragraph(i);
        let Some(layout) = layout else { return (if forward { p.end() } else { p.start }, Affinity::Downstream) };
        if forward && local >= p.len as usize {
            return match (i + 1 < self.doc.line_count()).then(|| self.doc.paragraph(i + 1)) {
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
        let moved = caret::step(layout, self.doc.slice(p.range()), Cursor::from_byte_index(layout, local, self.affinity), forward, word);
        (p.start + moved.index(), moved.affinity())
    }

    pub(super) fn vertical(&mut self, down: bool) -> (usize, Affinity) {
        if self.single_line {
            return if down { (self.doc.len(), Affinity::Downstream) } else { (0, Affinity::Downstream) };
        }
        let caret = self.caret();
        let i = self.doc.at_offset(self.focus);
        let band = |j: usize| {
            let p = self.doc.paragraph(j);
            let top = self.shift().y + self.doc.origin(j).y;
            (top, top + p.laid().map_or(self.doc.bottom(j) - self.doc.top(j), |layout| f64::from(layout.height())))
        };
        let y = match (down, band(i)) {
            (true, (_, bottom)) if caret.y1 + 1.0 >= bottom && i + 1 < self.doc.line_count() => band(i + 1).0 + 1.0,
            (false, (top, _)) if caret.y0 - 1.0 <= top && i > 0 => band(i - 1).1 - 1.0,
            (true, _) => caret.y1 + 1.0,
            (false, _) => caret.y0 - 1.0,
        };
        self.at_column(y)
    }

    fn at_column(&mut self, y: f64) -> (usize, Affinity) {
        let x = *self.column.get_or_insert(self.caret().x0);
        self.hit(Point::new(x, y))
    }

    pub fn page(this: &mut WidgetMut<'_, Self>, down: bool, dy: f64, extend: bool) {
        let editor = &mut *this.widget;
        if !editor.laid_out() {
            return;
        }
        let target = if dy.abs() < 0.5 {
            (if down { editor.doc.len() } else { 0 }, Affinity::Downstream)
        } else {
            let caret = editor.caret();
            editor.at_column((caret.y0 + caret.y1) / 2.0 + dy)
        };
        editor.move_to(target, extend);
        editor.skip_tables(down);
        Self::repaint(this);
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
