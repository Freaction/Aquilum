use masonry::core::keyboard::KeyboardEvent;

use super::*;
use crate::ui::editor::embed::View;
use crate::ui::editor::table::{Hit, Outcome, Session, Table, TableView};

impl TextEditor {
    fn table_view(&self, start: usize) -> Option<&TableView> {
        self.doc.table_at(start).map(|(_, view)| view)
    }

    fn table_under(&self, point: Point) -> Option<(usize, Point)> {
        if self.doc.markdown.is_none() || !self.laid_out() {
            return None;
        }
        let p = self.doc.paragraph(self.doc.at_y(point.y.max(0.0)));
        match (p.fold, p.embed.as_deref().map(|e| &e.view)) {
            (Fold::Head, Some(View::Table(_))) => Some((p.start, Point::new(point.x - self.shift().x, point.y - p.top))),
            _ => None,
        }
    }

    fn session(&self, start: usize) -> Session {
        Session { scroll_x: self.table_scrolls.get(&start).copied().unwrap_or(0.0), ..Session::new(start) }
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
                ctx.submit_action::<TextAction>(TextAction::Changed(self.doc.text().to_owned()));
            }
            Outcome::Leave => {
                self.leave_table();
            }
            Outcome::Remove => {
                self.remove_table();
                ctx.submit_action::<TextAction>(TextAction::Changed(self.doc.text().to_owned()));
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
        if self.doc.text()[range.clone()] == text {
            return;
        }
        let keep = (self.anchor, self.focus);
        self.replace(range, &text, Kind::Table);
        (self.anchor, self.focus) = keep;
        self.anchor = self.anchor.min(self.doc.len());
        self.focus = self.focus.min(self.doc.len());
    }

    fn remove_table(&mut self) {
        let Some(session) = self.table.take() else { return };
        let Some((range, _)) = self.doc.table_at(session.start) else { return };
        let end = (range.end + 1).min(self.doc.len());
        self.replace(range.start..end, "", Kind::Other);
        self.doc.editing = None;
    }

    pub(super) fn table_context(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((start, local)) = self.table_under(point) else { return false };
        if self.table.as_ref().is_none_or(|s| s.start != start) {
            self.table = Some(self.session(start));
        }
        let Some(view) = self.doc.table_at(start).map(|(_, v)| v) else { return false };
        let Some(session) = self.table.as_mut() else { return false };
        let crate::ui::editor::table::Hit::Cell(r, c, _) = view.hit(local, session.scroll_x) else { return true };
        let commands = session.context(view, r, c);
        if let Some(resolver) = &self.doc.resolver {
            let at = ctx.window_transform() * point;
            resolver.request(crate::ui::editor::resolve::Request::TableMenu { at, commands });
        }
        self.sync_editing();
        ctx.request_focus();
        ctx.request_layout();
        ctx.request_render();
        ctx.set_handled();
        true
    }

    pub fn table_command(this: &mut WidgetMut<'_, Self>, command: crate::ui::editor::table::Command) {
        let widget = &mut *this.widget;
        let Some(start) = widget.table.as_ref().map(|s| s.start) else { return };
        let Some(view) = widget.doc.table_at(start).map(|(_, v)| v) else { return };
        let Some(session) = widget.table.as_mut() else { return };
        let outcome = session.run(view, command);
        match outcome {
            Outcome::Write(model) => widget.write_table(&model),
            Outcome::Remove => widget.remove_table(),
            _ => {}
        }
        widget.sync_editing();
        this.ctx.submit_action::<TextAction>(TextAction::Changed(widget.doc.text().to_owned()));
        this.ctx.request_layout();
        this.ctx.request_render();
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

    pub(super) fn table_press(&mut self, ctx: &mut EventCtx<'_>, point: Point, shift: bool, new_tab: bool) -> bool {
        let Some((start, local)) = self.table_under(point) else {
            if self.table_active() {
                self.leave_table();
                self.sync_editing();
                ctx.request_layout();
            }
            return false;
        };
        if self.table.as_ref().is_none_or(|s| s.start != start) {
            self.table = Some(self.session(start));
        }
        let Some(view) = self.doc.table_at(start).map(|(_, v)| v) else { return false };
        if let crate::ui::editor::table::Hit::Cell(r, c, at) = view.hit(local, self.table.as_ref().map_or(0.0, |s| s.scroll_x))
            && let Some(cell) = view.cell(r, c).filter(|cell| cell.link && self.doc.editing != Some((start, (r, c))))
            && at.x >= 0.0
            && at.x <= f64::from(cell.layout.width())
            && at.y >= 0.0
            && at.y <= f64::from(cell.layout.height())
            && let Some(resolver) = &self.doc.resolver
        {
            let value = view.model.value(r, c).trim();
            let target = value.trim_start_matches("[[").trim_end_matches("]]").split('|').next().unwrap_or_default().to_owned();
            resolver.request(crate::ui::editor::resolve::Request::Open { target, wiki: true, new_tab });
            ctx.set_handled();
            return true;
        }
        let Some(session) = self.table.as_mut() else { return false };
        let outcome = session.press(view, local, shift);
        ctx.request_focus();
        ctx.capture_pointer();
        self.apply(ctx, outcome);
        true
    }

    pub(super) fn table_move(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let dragging = self.table.as_ref().is_some_and(|s| s.drag != crate::ui::editor::table::Drag::None);
        let start = match (dragging, self.table_under(point)) {
            (true, _) => self.table.as_ref().map(|s| s.start),
            (false, Some((start, _))) => Some(start),
            (false, None) => {
                if let Some(s) = self.table.as_mut()
                    && s.hover.take().is_some()
                {
                    s.line = crate::ui::editor::table::Line::None;
                    self.hovered_table = None;
                    ctx.request_render();
                }
                self.hovered_table = None;
                return false;
            }
        };
        let Some(start) = start else { return false };
        if self.table.as_ref().is_none_or(|s| s.start != start) {
            if self.table_active() {
                return false;
            }
            self.table = Some(self.session(start));
        }
        if self.hovered_table != Some(start) {
            self.hovered_table = Some(start);
            ctx.request_render();
        }
        let top = self.doc.paragraph(self.doc.at_offset(start)).top;
        let local = Point::new(point.x - self.shift().x, point.y - top);
        let Some(view) = self.doc.table_at(start).map(|(_, v)| v) else { return false };
        let Some(session) = self.table.as_mut() else { return false };
        let outcome = session.moved(view, local);
        self.apply(ctx, outcome);
        dragging
    }

    pub(super) fn table_release(&mut self, ctx: &mut EventCtx<'_>) -> bool {
        let Some(start) = self.table.as_ref().filter(|s| s.drag != crate::ui::editor::table::Drag::None).map(|s| s.start) else { return false };
        let Some(view) = self.doc.table_at(start).map(|(_, v)| v) else { return false };
        let Some(session) = self.table.as_mut() else { return false };
        let outcome = session.release(view);
        self.apply(ctx, outcome);
        true
    }

    pub(super) fn table_scroll(&mut self, ctx: &mut EventCtx<'_>, point: Point, dx: f64) -> bool {
        let Some((start, _)) = self.table_under(point) else { return false };
        let Some(max) = self.table_view(start).map(TableView::max_scroll).filter(|m| *m > 0.0) else { return false };
        if self.table.as_ref().is_none_or(|s| s.start != start) {
            self.table = Some(self.session(start));
        }
        if let Some(s) = self.table.as_mut() {
            s.scroll_x = (s.scroll_x + dx).clamp(0.0, max);
        }
        self.remember_scroll();
        ctx.request_render();
        true
    }

    pub(super) fn table_key(&mut self, ctx: &mut EventCtx<'_>, event: &KeyboardEvent) -> bool {
        if !self.table_active() || event.state != KeyState::Down {
            return self.table_active();
        }
        let shift = event.modifiers.shift();
        let action = if cfg!(target_os = "macos") { event.modifiers.meta() } else { event.modifiers.ctrl() };
        let Some(start) = self.table.as_ref().map(|s| s.start) else { return false };
        let Some(view) = self.doc.table_at(start).map(|(_, v)| v) else {
            self.table = None;
            return false;
        };
        let Some(session) = self.table.as_mut() else { return false };
        if session.active.is_none() {
            let outcome = match &event.key {
                Key::Named(NamedKey::Delete | NamedKey::Backspace) => session.clear_range(view),
                Key::Named(NamedKey::Escape) => {
                    session.range = None;
                    Outcome::Repaint
                }
                _ => Outcome::None,
            };
            self.apply(ctx, outcome);
            ctx.set_handled();
            return true;
        }
        let mut clipboard = None;
        let outcome = match &event.key {
            Key::Character(c) if action && c.eq_ignore_ascii_case("a") => {
                session.select_all(view);
                Outcome::Repaint
            }
            Key::Character(c) if action && c.eq_ignore_ascii_case("c") => {
                clipboard = session.selected(view);
                Outcome::None
            }
            Key::Character(c) if action && c.eq_ignore_ascii_case("x") => {
                clipboard = session.selected(view);
                if clipboard.is_some() { session.insert(view, "") } else { Outcome::None }
            }
            Key::Character(c) if action && (c.eq_ignore_ascii_case("z") || c.eq_ignore_ascii_case("y")) => {
                self.leave_table();
                self.sync_editing();
                return false;
            }
            Key::Character(text) if !action => session.insert(view, text),
            Key::Named(NamedKey::Backspace) => session.delete(view, false),
            Key::Named(NamedKey::Delete) => session.delete(view, true),
            Key::Named(NamedKey::ArrowLeft) | Key::Named(NamedKey::ArrowRight) => {
                session.step(view, matches!(event.key, Key::Named(NamedKey::ArrowRight)), action, shift);
                Outcome::Repaint
            }
            Key::Named(NamedKey::Home) | Key::Named(NamedKey::ArrowUp) => {
                session.edge(view, false, shift);
                Outcome::Repaint
            }
            Key::Named(NamedKey::End) | Key::Named(NamedKey::ArrowDown) => {
                session.edge(view, true, shift);
                Outcome::Repaint
            }
            Key::Named(NamedKey::Enter) => session.down(view),
            Key::Named(NamedKey::Tab) => session.next(view, shift),
            Key::Named(NamedKey::Escape) => Outcome::Leave,
            _ => Outcome::None,
        };
        if let Some(text) = clipboard {
            ctx.set_clipboard(text);
        }
        self.apply(ctx, outcome);
        ctx.set_handled();
        true
    }

    pub(super) fn table_text(&mut self, ctx: &mut EventCtx<'_>, text: &str) -> bool {
        if !self.table.as_ref().is_some_and(|s| s.active.is_some()) {
            return false;
        }
        let Some(start) = self.table.as_ref().map(|s| s.start) else { return false };
        let Some(view) = self.doc.table_at(start).map(|(_, v)| v) else { return false };
        let Some(session) = self.table.as_mut() else { return false };
        let outcome = session.insert(view, text);
        self.apply(ctx, outcome);
        ctx.set_handled();
        true
    }

    pub(super) fn table_cursor(&self) -> Option<CursorIcon> {
        let session = self.table.as_ref()?;
        Some(match (session.drag, session.hover?) {
            (crate::ui::editor::table::Drag::Strip { moved: true, .. }, _) => CursorIcon::Grabbing,
            (_, Hit::Cell(..)) => CursorIcon::Text,
            (_, Hit::Column(_) | Hit::Row(_)) => CursorIcon::Grab,
            (_, Hit::ColumnBorder(_)) => CursorIcon::ColResize,
            (_, Hit::RowBorder(_)) => CursorIcon::RowResize,
            (_, Hit::AddRow | Hit::AddCol) => CursorIcon::Pointer,
            _ => CursorIcon::Default,
        })
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
            let half = height * CARET_LINE_SCALE / 2.0;
            let mid = at.y + height / 2.0;
            let caret = Rect::new(at.x, mid - half, at.x + size::CARET_WIDTH, mid + half) + origin;
            painter.fill(RoundedRect::from_rect(caret, size::CARET_RADIUS), tm.caret_color).draw();
        }
    }
}
