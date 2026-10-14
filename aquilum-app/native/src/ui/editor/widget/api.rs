use super::*;
use aquilum_core::markdown::line;

impl TextEditor {
    pub fn text(&self) -> &str {
        self.doc.text()
    }

    #[cfg(test)]
    pub fn caret_rect(&self) -> masonry::kurbo::Rect {
        self.caret()
    }

    pub fn caret_offset(&self) -> usize {
        self.selection().start
    }

    pub fn offset_at(&self, window_to_local: Affine, window: Point) -> usize {
        self.hit(window_to_local * window).0
    }

    pub fn anchor_at(&self, y: f64) -> Option<(usize, f64)> {
        if !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(y.max(0.0)));
        Some((p.start, y - p.top))
    }

    pub fn top_of(&self, offset: usize) -> Option<f64> {
        self.laid_out().then(|| self.doc.paragraph(self.doc.at_offset(offset.min(self.doc.len()))).top)
    }

    fn repaint(this: &mut WidgetMut<'_, Self>) {
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    fn edited(this: &mut WidgetMut<'_, Self>) {
        this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
        Self::repaint(this);
    }

    pub fn refresh_embeds(this: &mut WidgetMut<'_, Self>) {
        let id = this.ctx.widget_id();
        this.widget.poll_suggest(id);
        Self::repaint(this);
    }

    pub fn reset_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        let widget = &mut *this.widget;
        widget.videos.stop();
        widget.doc.set_text(text);
        widget.history.clear();
        widget.refind();
        widget.table_scrolls.clear();
        widget.compose = None;
        widget.extra_cursors.clear();
        widget.media = Default::default();
        (widget.table, widget.hovered_table, widget.hovered_link, widget.unit) = (None, None, None, None);
        let end = widget.doc.len();
        widget.anchor = text.floor_char_boundary(widget.anchor.min(end));
        widget.focus = text.floor_char_boundary(widget.focus.min(end));
        Self::repaint(this);
    }

    pub fn select_byte_range(this: &mut WidgetMut<'_, Self>, start: usize, end: usize) {
        let widget = &mut *this.widget;
        let len = widget.doc.len();
        widget.anchor = widget.doc.text().floor_char_boundary(start.min(len));
        widget.focus = widget.doc.text().floor_char_boundary(end.min(len));
        widget.column = None;
        Self::repaint(this);
    }

    pub fn insert_block(this: &mut WidgetMut<'_, Self>, at: usize, lines: &[String]) {
        let text = this.widget.doc.text();
        let at = at.min(text.len());
        let Range { start: line_start, end: line_end } = line::bounds(text, at);
        let prefix = if at == line_start { "" } else { "\n" };
        let tail = if text[at..line_end].trim().is_empty() { "" } else { "\n" };
        let insert = format!("{prefix}{}\n{tail}", lines.join("\n\n"));
        this.widget.replace(at..at, &insert, Kind::Other);
        Self::edited(this);
    }

    pub fn insert_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        let range = this.widget.selection();
        this.widget.replace(range, text, Kind::Other);
        Self::edited(this);
    }
}
