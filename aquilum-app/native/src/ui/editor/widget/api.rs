use super::*;
use crate::ui::editor::Change;

impl TextEditor {
    pub fn text(&self) -> String {
        self.doc.whole()
    }

    pub fn len(&self) -> usize {
        self.doc.len()
    }

    pub fn rope(&self) -> crop::Rope {
        self.doc.rope()
    }

    #[cfg(test)]
    pub fn caret_rect(&self) -> masonry::kurbo::Rect {
        self.caret()
    }

    #[cfg(test)]
    pub fn flashed(&self) -> Option<Range<usize>> {
        self.flash.as_ref().map(|(range, _)| range.clone())
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
        let i = self.doc.at_y(y.max(0.0));
        Some((self.doc.paragraph(i).start, y - self.doc.top(i)))
    }

    pub fn top_of(&self, offset: usize) -> Option<f64> {
        self.laid_out().then(|| self.doc.y_of(offset.min(self.doc.len())))
    }

    pub(super) fn repaint(this: &mut WidgetMut<'_, Self>) {
        if this.widget.grips.on() {
            this.ctx.request_compose();
        }
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    pub fn can_undo(&self, redo: bool) -> bool {
        self.history.has(redo)
    }

    pub fn undo(this: &mut WidgetMut<'_, Self>, redo: bool) {
        if this.widget.rewind(redo) {
            this.widget.follow_caret = true;
            Self::edited(this);
        }
    }

    pub fn keep_caret(this: &mut WidgetMut<'_, Self>) {
        this.widget.follow_caret = true;
        Self::repaint(this);
    }

    pub(super) fn edited(this: &mut WidgetMut<'_, Self>) {
        this.ctx.submit_action::<TextAction>(this.widget.doc.changed());
        Self::repaint(this);
    }

    pub fn refresh_embeds(this: &mut WidgetMut<'_, Self>) {
        let id = this.ctx.widget_id();
        this.widget.poll_suggest(id);
        Self::repaint(this);
    }

    pub fn reset_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        Self::reset_rope(this, crop::Rope::from(text));
    }

    pub fn reset_rope(this: &mut WidgetMut<'_, Self>, text: crop::Rope) {
        let widget = &mut *this.widget;
        widget.videos.stop();
        widget.doc.set_rope(text);
        widget.history = Default::default();
        widget.refind(&[]);
        widget.embed_scrolls.clear();
        widget.compose = None;
        widget.extra_cursors.clear();
        widget.media = Default::default();
        (widget.table, widget.hovered_table, widget.hovered_link, widget.unit) = (None, None, None, None);
        widget.anchor = widget.doc.floor(widget.anchor);
        widget.focus = widget.doc.floor(widget.focus);
        Self::repaint(this);
    }

    pub fn select_byte_range(this: &mut WidgetMut<'_, Self>, start: usize, end: usize) {
        let widget = &mut *this.widget;
        widget.anchor = widget.doc.floor(start);
        widget.focus = widget.doc.floor(end);
        widget.column = None;
        Self::repaint(this);
    }

    pub fn insert_media(this: &mut WidgetMut<'_, Self>, lines: &[String], at: Option<Point>) -> bool {
        if let Some(at) = at {
            let offset = this.widget.offset_at(this.ctx.window_transform().inverse(), at);
            this.widget.anchor = offset;
            this.widget.focus = offset;
            this.widget.column = None;
        }
        if !this.widget.allows_media() {
            return false;
        }
        let text = lines.join("\n");
        if this.widget.table.as_ref().is_some_and(|s| s.active.is_some()) {
            let Some((session, view)) = this.widget.table_parts() else { return false };
            return match session.insert(view, &text) {
                crate::ui::editor::table::Outcome::Write(model) => {
                    this.widget.write_table(&model);
                    this.widget.sync_editing();
                    Self::edited(this);
                    true
                }
                _ => false,
            };
        }
        if this.widget.table_active() {
            return false;
        }
        Self::insert_text(this, &text);
        true
    }

    pub fn apply_external(this: &mut WidgetMut<'_, Self>, range: Range<usize>, insert: &str, undoable: bool) {
        let widget = &mut *this.widget;
        let before = (widget.anchor, widget.focus);
        let change = widget.apply_change(range, insert.to_owned());
        (widget.anchor, widget.focus) = (change.map(widget.anchor), change.map(widget.focus));
        widget.remember(vec![change], before, Kind::Other, undoable);
        Self::repaint(this);
    }

    pub fn change_outside(this: &mut WidgetMut<'_, Self>, change: Change, undoable: bool) {
        let widget = &mut *this.widget;
        widget.remember(vec![change], (widget.anchor, widget.focus), Kind::Outside, undoable);
    }

    pub fn insert_text(this: &mut WidgetMut<'_, Self>, text: &str) {
        let range = this.widget.selection();
        this.widget.replace(range, text, Kind::Other);
        Self::edited(this);
    }

    pub fn paste_markdown(this: &mut WidgetMut<'_, Self>, text: &str) -> bool {
        let widget = &mut *this.widget;
        let in_code = widget.doc.paragraph(widget.doc.at_offset(widget.focus)).block.is_code();
        if widget.single_line || widget.doc.markdown.is_none() || widget.table_active() || in_code {
            return false;
        }
        Self::insert_text(this, text);
        true
    }
}
