use super::*;

impl TextEditor {
    pub(super) fn selection(&self) -> Range<usize> {
        self.anchor.min(self.focus)..self.anchor.max(self.focus)
    }

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

    pub(super) fn move_to(&mut self, (offset, affinity): (usize, Affinity), extend: bool) {
        self.focus = offset;
        self.affinity = affinity;
        if !extend {
            self.anchor = offset;
        }
    }

    pub(super) fn replace(&mut self, range: Range<usize>, text: &str, kind: Kind) {
        let before = (self.anchor, self.focus);
        let removed = self.doc.text()[range.clone()].to_owned();
        self.doc.replace(range.clone(), text);
        self.anchor = range.start + text.len();
        self.focus = self.anchor;
        self.affinity = Affinity::Downstream;
        self.column = None;
        let edit = Edit { at: range.start, removed, inserted: text.to_owned(), before, after: (self.anchor, self.focus), kind, time: std::time::Instant::now() };
        self.history.record(edit);
        if self.doc.markdown.is_some() {
            self.renumber();
        }
        self.reveal_edited_image();
        self.refind();
    }

    fn reveal_edited_image(&mut self) {
        let p = self.doc.paragraph(self.doc.at_offset(self.focus));
        if matches!(p.block, crate::ui::editor::markdown::Block::Image) {
            self.doc.source = Some(p.start);
        }
    }

    fn renumber(&mut self) {
        let Some(last) = self.history.last_mut() else { return };
        let Some((range, numbers)) = super::outline::renumber(self.doc.text(), last.at..last.at + last.inserted.len()) else { return };
        let text = self.doc.text();
        let (lo, hi) = (last.at.min(range.start), (last.at + last.inserted.len()).max(range.end));
        let mut original = text[lo..hi].to_owned();
        original.replace_range(last.at - lo..last.at - lo + last.inserted.len(), &last.removed);
        let mut updated = text[lo..hi].to_owned();
        updated.replace_range(range.start - lo..range.end - lo, &numbers);
        let shift = |p: usize| if p >= range.end { p + numbers.len() - range.len() } else { p.min(range.start + numbers.len()) };
        let selection = (shift(self.anchor), shift(self.focus));
        (last.at, last.removed, last.inserted, last.after) = (lo, original, updated, selection);
        self.doc.replace(range, &numbers);
        (self.anchor, self.focus) = selection;
    }

    pub(super) fn insert(&mut self, text: &str, kind: Kind) {
        if self.single_line && text.contains(['\r', '\n']) {
            let flat = text.replace("\r\n", " ").replace(['\r', '\n'], " ");
            self.replace(self.selection(), &flat, kind);
        } else {
            self.replace(self.selection(), text, kind);
        }
    }

    pub(super) fn delete(&mut self, forward: bool, word: bool) {
        let range = self.selection();
        if !range.is_empty() {
            self.replace(range, "", Kind::Other);
            return;
        }
        let (target, _) = self.step(forward, word);
        let range = self.focus.min(target)..self.focus.max(target);
        if !range.is_empty() {
            self.replace(range, "", Kind::Deleting);
        }
    }

    pub(super) fn undo(&mut self, redo: bool) -> bool {
        self.leave_table();
        let edit = if redo { self.history.redo() } else { self.history.undo() };
        let Some(edit) = edit else { return false };
        let (from, to, selection) = if redo { (&edit.removed, &edit.inserted, edit.after) } else { (&edit.inserted, &edit.removed, edit.before) };
        self.doc.replace(edit.at..edit.at + from.len(), to);
        (self.anchor, self.focus) = selection;
        self.affinity = Affinity::Downstream;
        self.column = None;
        self.reveal_edited_image();
        self.refind();
        true
    }

    #[cfg(test)]
    pub fn selected(&self) -> String {
        self.selected_text().unwrap_or_default()
    }

    #[cfg(test)]
    pub fn image_source_shown(&self) -> bool {
        self.doc.source.is_some()
    }

    #[cfg(test)]
    pub fn brush_at(&self, offset: usize) -> Option<BrushIndex> {
        let (_, local, layout) = self.local(offset);
        layout?.lines().flat_map(|line| line.items().collect::<Vec<_>>()).find_map(|item| match item {
            masonry::parley::PositionedLayoutItem::GlyphRun(run) if run.run().text_range().contains(&local) => Some(run.style().brush.clone()),
            _ => None,
        })
    }

    pub fn selected_text(&self) -> Option<String> {
        let range = self.selection();
        (!range.is_empty()).then(|| self.doc.text()[range].to_owned())
    }

    fn unit_at(&self, point: Point, count: u8) -> Range<usize> {
        let (offset, _) = self.hit(point);
        let p = self.doc.paragraph(self.doc.at_offset(offset));
        let local = point - self.shift() - p.origin();
        match count {
            2 => {
                let Some(layout) = p.layout.as_deref() else { return offset..offset };
                let r = Selection::word_from_point(layout, local.x as f32, local.y as f32).text_range();
                p.start + r.start..p.start + r.end
            }
            3 => p.range(),
            _ => offset..offset,
        }
    }

    pub(super) fn select_at(&mut self, point: Point, count: u8, extend: bool) {
        let (offset, affinity) = self.hit(point);
        self.unit = None;
        if extend {
            self.move_to((offset, affinity), true);
            return;
        }
        let range = self.unit_at(point, count.min(3));
        if count >= 2 {
            self.unit = Some((count.min(3), range.clone()));
        }
        self.anchor = range.start;
        self.focus = range.end;
        self.affinity = affinity;
        self.column = None;
    }

    pub(super) fn drag_select(&mut self, point: Point) {
        let Some((count, first)) = self.unit.clone() else {
            let target = self.hit(point);
            self.move_to(target, true);
            return;
        };
        let under = self.unit_at(point, count);
        (self.anchor, self.focus) = if under.start < first.start { (first.end, under.start) } else { (first.start, first.end.max(under.end)) };
        self.column = None;
    }

    pub(super) fn refresh(&mut self, ctx: &mut EventCtx<'_>) {
        if self.doc.markdown.is_some() {
            ctx.request_layout();
        }
        self.reveal_caret();
        if self.laid_out() {
            let caret = self.caret();
            ctx.set_ime_area(caret);
            ctx.request_scroll_to(caret);
        }
        ctx.request_render();
    }

    pub(super) fn changed(&mut self, ctx: &mut EventCtx<'_>) {
        ctx.submit_action::<TextAction>(self.doc.changed());
        ctx.request_layout();
        ctx.request_render();
    }

    pub(super) fn ensure_layout(&mut self, fcx: &mut masonry::parley::FontContext, lcx: &mut masonry::parley::LayoutContext<BrushIndex>, width: f64) {
        let _zone = aq_trace::zone("редактор: вёрстка");
        self.width = width;
        let compose = self.compose.as_ref().map(|(text, _)| (self.focus, text.as_str()));
        let wrap = (!self.single_line).then(|| (width - size::CARET_WIDTH * 2.0).max(0.0) as f32);
        let focus = self.focus;
        self.doc.source = self.doc.source.filter(|&source| {
            let p = self.doc.paragraph(self.doc.at_offset(source));
            p.start == source && (p.start..=p.end()).contains(&focus)
        });
        if self.single_line {
            self.doc.keep = (f64::NEG_INFINITY, f64::INFINITY);
        }
        self.doc.pins = [self.anchor, self.focus].into_iter().chain(self.extra_cursors.iter().flat_map(|&(a, f)| [a, f])).collect();
        self.doc.layout(fcx, lcx, &self.styles, wrap, compose, self.focused.then_some(self.focus));
    }

}

impl TextEditor {
    pub fn caret_offset(&self) -> usize {
        self.selection().start
    }

    /// Позиция в тексте под точкой в координатах окна (логических).
    pub fn offset_at(&self, window_to_local: masonry::kurbo::Affine, window: Point) -> usize {
        self.hit(window_to_local * window).0
    }

    /// Вставляет строки отдельным блоком в `at` (как `insertImageLines`): с новой строки, между
    /// строками пустая строка, после блока — перевод строки.
    pub fn insert_block(this: &mut WidgetMut<'_, Self>, at: usize, lines: &[String]) {
        let text = this.widget.doc.text();
        let at = at.min(text.len());
        let line_start = text[..at].rfind('\n').map_or(0, |i| i + 1);
        let line_end = text[at..].find('\n').map_or(text.len(), |i| at + i);
        let prefix = if at == line_start { "" } else { "\n" };
        let tail = if text[at..line_end].trim().is_empty() { "" } else { "\n" };
        let insert = format!("{prefix}{}\n{tail}", lines.join("\n\n"));
        this.widget.replace(at..at, &insert, Kind::Other);
        this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    pub fn insert_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        let range = this.widget.selection();
        this.widget.replace(range, text, Kind::Other);
        this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
        this.ctx.request_layout();
        this.ctx.request_render();
    }
}
