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
        Session { scroll_x: self.embed_scrolls.get(&start).copied().unwrap_or(0.0), ..Session::new(start) }
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

    pub(super) fn table_cell(&self) -> Option<(&str, &Session)> {
        let session = self.table.as_ref()?;
        let (row, col) = session.active?;
        let (_, view) = self.doc.table_at(session.start)?;
        Some((view.model.value(row, col), session))
    }

    #[cfg(target_os = "android")]
    pub(super) fn table_select(&mut self, anchor: usize, cursor: usize) -> bool {
        let Some((session, view)) = self.table_parts() else { return false };
        let Some((row, col)) = session.active else { return false };
        let value = view.model.value(row, col);
        (session.anchor, session.caret) = (value.floor_char_boundary(anchor), value.floor_char_boundary(cursor));
        true
    }

    pub(super) fn embed_scroll_x(&self, start: usize) -> f64 {
        match &self.table {
            Some(session) if session.start == start => session.scroll_x,
            _ => self.embed_scrolls.get(&start).copied().unwrap_or(0.0),
        }
    }

    fn remember_scroll(&mut self) {
        if let Some(session) = &self.table {
            self.embed_scrolls.insert(session.start, session.scroll_x);
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
            Outcome::None | Outcome::Copy(_) => return,
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
        let i = self.doc.at_offset(range.start);
        let prefix = &self.doc.line(i)[..self.doc.frame(i).lead];
        let text = format!("{prefix}{}", model.serialize().replace('\n', &format!("\n{prefix}")));
        if self.doc.slice(range.clone()) != text {
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
        session.compose = None;
        match session.run(view, command) {
            Outcome::Write(model) => widget.write_table(&model),
            Outcome::Remove => widget.remove_table(),
            Outcome::Copy(text) => this.ctx.set_clipboard(text),
            _ => {}
        }
        widget.sync_editing();
        Self::edited(this);
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

    pub(super) fn table_caret(&self) -> Option<(Rect, Vec<Rect>)> {
        let session = self.table.as_ref()?;
        let (range, view) = self.doc.table_at(session.start)?;
        let i = self.doc.at_offset(range.start);
        let (at, height, selection) = session.caret_geometry(view)?;
        let origin = Vec2::new(self.embed_x(i), self.doc.top(i));
        let caret = Rect::new(at.x, at.y, at.x + size::CARET_WIDTH, at.y + height) + origin;
        Some((caret, selection.into_iter().map(|rect| rect + origin).collect()))
    }

    pub(super) fn paint_table_caret(&self, painter: &mut Painter<'_>) {
        let Some((caret, selection)) = self.table_caret() else { return };
        let tm = theme::current();
        for rect in selection {
            painter.fill(rect, tm.selection_bg).draw();
        }
        if self.caret_visible {
            painter.fill(RoundedRect::from_rect(caret, size::CARET_RADIUS), tm.caret_color).draw();
        }
    }
}
