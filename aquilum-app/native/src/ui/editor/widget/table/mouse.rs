use super::*;
use crate::ui::editor::markdown::Link;
use crate::ui::editor::resolve::Request;
use crate::ui::editor::table::{Drag, Hit, Line};
use crate::ui::keymap::gesture::GRIP;

impl TextEditor {
    pub(in super::super) fn table_context(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((start, local)) = self.table_under(point) else { return false };
        self.open_session(start);
        let Some((session, view)) = self.table_parts() else { return false };
        let Hit::Cell(r, c, _) = view.hit(local, session.scroll_x, GRIP) else { return true };
        let commands = session.context(view, r, c);
        if let Some(resolver) = &self.doc.resolver {
            let at = ctx.window_transform() * point;
            resolver.request(Request::TableMenu { at, commands });
        }
        self.sync_editing();
        ctx.request_focus();
        ctx.request_layout();
        ctx.request_render();
        ctx.set_handled();
        true
    }

    pub(in super::super) fn table_press(&mut self, ctx: &mut EventCtx<'_>, point: Point, shift: bool, new_tab: bool, count: u8) -> bool {
        let Some((start, local)) = self.table_under(point) else {
            if self.table_active() {
                self.leave_table();
                self.sync_editing();
                ctx.request_layout();
            }
            return false;
        };
        self.open_session(start);
        if let Some(link) = self.link_under(start, local)
            && let Some(resolver) = &self.doc.resolver
        {
            resolver.request(Request::Open { target: link.target.clone(), wiki: link.wiki, new_tab });
            ctx.set_handled();
            return true;
        }
        let Some((session, view)) = self.table_parts() else { return false };
        if view.hit(local, session.scroll_x, GRIP) != Hit::Bar {
            ctx.request_focus();
        }
        let outcome = session.press(view, local, shift, count, GRIP);
        ctx.capture_pointer();
        self.apply(ctx, outcome);
        true
    }

    pub(in super::super) fn table_move(&mut self, ctx: &mut EventCtx<'_>, point: Point, pressed: bool) -> bool {
        if !pressed {
            self.table_release(ctx);
        }
        let dragging = self.table.as_ref().is_some_and(|s| s.drag != Drag::None);
        let start = match (dragging, self.table_under(point)) {
            (true, _) => self.table.as_ref().map(|s| s.start),
            (false, Some((start, _))) => Some(start),
            (false, None) => {
                if self.table_unhover() {
                    ctx.request_render();
                }
                return false;
            }
        };
        let Some(start) = start else { return false };
        if !self.is_session(start) && self.table_active() {
            return false;
        }
        self.open_session(start);
        if self.hovered_table != Some(start) {
            self.hovered_table = Some(start);
            ctx.request_render();
        }
        let local = self.embed_local_at(start, point);
        let Some((session, view)) = self.table_parts() else { return dragging };
        let outcome = session.moved(view, local, GRIP);
        self.apply(ctx, outcome);
        dragging
    }

    pub(in super::super) fn table_unhover(&mut self) -> bool {
        let session = self.table.as_mut().filter(|s| s.drag == Drag::None).is_some_and(|s| s.hover.take().is_some() | (std::mem::replace(&mut s.line, Line::None) != Line::None));
        session | self.hovered_table.take().is_some()
    }

    pub(in super::super) fn table_release(&mut self, ctx: &mut EventCtx<'_>) {
        if self.table.as_ref().is_none_or(|s| s.drag == Drag::None) {
            return;
        }
        let Some((session, view)) = self.table_parts() else { return self.leave_table() };
        let outcome = session.release(view);
        self.apply(ctx, outcome);
    }

    pub(in super::super) fn table_scroll(&mut self, ctx: &mut EventCtx<'_>, point: Point, dx: f64) -> bool {
        let Some((start, local)) = self.table_under(point) else { return false };
        if self.doc.table_at(start).is_none_or(|(_, view)| view.max_scroll() <= 0.0) {
            return false;
        }
        self.open_session(start);
        if let Some((session, view)) = self.table_parts() {
            session.scroll_x = (session.scroll_x + dx).clamp(0.0, view.max_scroll());
            session.moved(view, local, GRIP);
        }
        self.remember_scroll();
        ctx.request_render();
        true
    }

    fn link_under(&self, start: usize, local: Point) -> Option<&Link> {
        let (_, view) = self.doc.table_at(start)?;
        self.cell_link(view, start, view.hit(local, self.embed_scroll_x(start), GRIP))
    }

    fn cell_link<'v>(&self, view: &'v TableView, start: usize, hit: Hit) -> Option<&'v Link> {
        let Hit::Cell(r, c, at) = hit else { return None };
        let cell = view.cell(r, c).filter(|_| self.doc.editing != Some((start, (r, c))))?;
        cell.decor.link_at(&cell.layout, at.x, at.y)
    }

    pub(in super::super) fn table_cursor(&self) -> Option<CursorIcon> {
        let session = self.table.as_ref()?;
        Some(match (session.drag, session.hover?) {
            (Drag::Strip { moved: true, .. }, _) => CursorIcon::Grabbing,
            (Drag::Bar, _) | (_, Hit::Bar) => session.bar.cursor(),
            (_, hit @ Hit::Cell(..)) if self.doc.table_at(session.start).is_some_and(|(_, view)| self.cell_link(view, session.start, hit).is_some()) => CursorIcon::Pointer,
            (_, Hit::Cell(..)) => CursorIcon::Text,
            (_, Hit::Column(_) | Hit::Row(_)) => CursorIcon::Move,
            (_, Hit::ColumnBorder(_)) => CursorIcon::ColResize,
            (_, Hit::RowBorder(_)) => CursorIcon::RowResize,
            (_, Hit::AddRow | Hit::AddCol) => CursorIcon::Pointer,
            _ => CursorIcon::Default,
        })
    }
}
