use super::*;
use crate::ui::editor::embed::View;
use crate::ui::editor::table::{Command, Outcome, Session, Table, TableView};

mod keys;
mod mouse;

impl TextEditor {
    fn table_under(&self, point: Point) -> Option<(usize, Point)> {
        match self.head_embed_at(point)? {
            (p, View::Table(_), local) => Some((p.start, local)),
            _ => None,
        }
    }

    fn session(&self, start: usize) -> Session {
        Session { scroll_x: self.table_scrolls.get(&start).copied().unwrap_or(0.0), ..Session::new(start) }
    }

    fn is_session(&self, start: usize) -> bool {
        self.table.as_ref().is_some_and(|s| s.start == start)
    }

    fn open_session(&mut self, start: usize) {
        if !self.is_session(start) {
            self.table = Some(self.session(start));
        }
    }

    fn table_parts(&mut self) -> Option<(&mut Session, &TableView)> {
        let session = self.table.as_mut()?;
        let (_, view) = self.doc.table_at(session.start)?;
        Some((session, view))
    }

    pub(super) fn table_scroll_x(&self, start: usize) -> f64 {
        match &self.table {
            Some(session) if session.start == start => session.scroll_x,
            _ => self.table_scrolls.get(&start).copied().unwrap_or(0.0),
        }
    }

    fn remember_scroll(&mut self) {
        if let Some(session) = &self.table {
            self.table_scrolls.insert(session.start, session.scroll_x);
        }
    }

    pub(super) fn table_active(&self) -> bool {
        self.table.as_ref().is_some_and(|s| s.active.is_some() || s.range.is_some())
    }

    fn sync_editing(&mut self) {
        self.doc.editing = self.table.as_ref().and_then(|s| s.active.map(|cell| (s.start, cell)));
    }

    fn apply(&mut self, ctx: &mut EventCtx<'_>, outcome: Outcome) {
        self.remember_scroll();
        match outcome {
            Outcome::None => return,
            Outcome::Repaint => {}
            Outcome::Write(model) => {
                self.write_table(&model);
                ctx.submit_action::<TextAction>(self.doc.changed());
            }
            Outcome::Leave => {
                self.leave_table();
            }
            Outcome::Remove => {
                self.remove_table();
                ctx.submit_action::<TextAction>(self.doc.changed());
            }
        }
        self.sync_editing();
        self.restart_blink(ctx);
        ctx.request_layout();
        ctx.request_render();
    }

    fn write_table(&mut self, model: &Table) {
        let Some(start) = self.table.as_ref().map(|s| s.start) else { return };
        let Some((range, _)) = self.doc.table_at(start) else { return };
        let text = model.serialize();
        if self.doc.text()[range.clone()] != text {
            self.dispatch(vec![(range, text)], None, Kind::Block);
        }
    }

    fn remove_table(&mut self) {
        let Some(session) = self.table.take() else { return };
        let Some((range, _)) = self.doc.table_at(session.start) else { return };
        let end = (range.end + 1).min(self.doc.len());
        self.replace(range.start..end, "", Kind::Other);
        self.doc.editing = None;
    }

    pub fn table_command(this: &mut WidgetMut<'_, Self>, command: Command) {
        let widget = &mut *this.widget;
        let Some((session, view)) = widget.table_parts() else { return };
        match session.run(view, command) {
            Outcome::Write(model) => widget.write_table(&model),
            Outcome::Remove => widget.remove_table(),
            _ => {}
        }
        widget.sync_editing();
        this.ctx.submit_action::<TextAction>(widget.doc.changed());
        this.ctx.request_layout();
        this.ctx.request_render();
    }

    pub fn insert_table(this: &mut WidgetMut<'_, Self>, cols: usize, rows: usize) {
        let text = format!("{}\n", Table::empty(cols, rows).serialize());
        Self::insert_text(this, &text);
    }

    pub(super) fn leave_table(&mut self) {
        if let Some(session) = self.table.take()
            && let Some((range, _)) = self.doc.table_at(session.start)
        {
            let after = (range.end + 1).min(self.doc.len());
            self.anchor = after;
            self.focus = after;
        }
        self.doc.editing = None;
    }

    pub(super) fn skip_tables(&mut self, forward: bool) {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return;
        }
        let i = self.doc.at_offset(self.focus);
        let p = self.doc.paragraph(i);
        if !matches!(p.block, Block::Table { .. }) {
            return;
        }
        let mut head = i;
        while head > 0 && !matches!(self.doc.paragraph(head).block, Block::Table { first: true, .. }) {
            head -= 1;
        }
        let Some((range, _)) = self.doc.table_at(self.doc.paragraph(head).start) else { return };
        let target = if forward { (range.end + 1).min(self.doc.len()) } else { range.start.saturating_sub(1) };
        self.focus = target;
        self.anchor = target;
    }

    pub(super) fn paint_table_caret(&self, painter: &mut Painter<'_>) {
        let Some(session) = self.table.as_ref().filter(|s| s.active.is_some()) else { return };
        let Some((range, view)) = self.doc.table_at(session.start) else { return };
        let top = self.doc.paragraph(self.doc.at_offset(range.start)).top;
        let Some((at, height, selection)) = session.caret_geometry(view) else { return };
        let origin = Vec2::new(self.shift().x, top);
        let tm = theme::current();
        for rect in selection {
            painter.fill(rect + origin, tm.selection_bg).draw();
        }
        if self.caret_visible {
            let half = height / 2.0;
            let mid = at.y + height / 2.0;
            let caret = Rect::new(at.x, mid - half, at.x + size::CARET_WIDTH, mid + half) + origin;
            painter.fill(RoundedRect::from_rect(caret, size::CARET_RADIUS), tm.caret_color).draw();
        }
    }
}
