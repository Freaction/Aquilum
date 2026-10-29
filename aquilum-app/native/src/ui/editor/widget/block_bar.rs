use super::*;
use crate::ui::editor::embed::View;
use crate::ui::scroll::bar::{Scrollbar, Span};

impl TextEditor {
    pub(super) fn block_span(&self, start: usize) -> Option<Span> {
        let i = self.doc.at_offset(start);
        let p = self.doc.paragraph(i);
        match (p.fold, p.embed.map(|embed| &embed.view)) {
            (Fold::Head, Some(View::Drawing(drawing))) => drawing.scrollbar(self.embed_x(i), self.doc.top(i), self.embed_scroll_x(start)),
            (Fold::Head, _) => None,
            _ if p.block.is_code() => self.code_span(&self.code(i, self.doc.markdown.as_ref()?)),
            _ => None,
        }
    }

    fn block_under(&self, point: Point) -> Option<(usize, Span)> {
        let p = self.doc.paragraph(self.index_at(point)?);
        let start = if p.fold == Fold::Head { p.start } else { self.doc.paragraph(self.code_at(point)?.first).start };
        Some((start, self.block_span(start)?))
    }

    fn scroll_block(&mut self, start: usize, x: f64) {
        let i = self.doc.at_offset(start);
        if self.doc.paragraph(i).fold == Fold::Head {
            self.embed_scrolls.insert(start, x);
        } else if let Some(fonts) = self.doc.markdown.as_ref() {
            let code = self.code(i, fonts);
            self.scroll_code(&code, x);
        }
    }

    fn bar_of(&self, start: usize) -> Scrollbar {
        self.block_bar.filter(|(from, _)| *from == start).map_or(Scrollbar::new(Axis::Horizontal), |(_, bar)| bar)
    }

    pub(super) fn block_scroll(&mut self, ctx: &mut EventCtx<'_>, point: Point, dx: f64) -> bool {
        let Some((start, span)) = self.block_under(point) else { return false };
        self.scroll_block(start, (span.offset + dx).clamp(0.0, span.max()));
        ctx.request_render();
        true
    }

    pub(super) fn bar_press(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((start, span)) = self.block_under(point) else { return false };
        let mut bar = self.bar_of(start);
        let Some(offset) = bar.press(point, &span) else { return false };
        self.block_bar = Some((start, bar));
        self.scroll_block(start, offset);
        ctx.capture_pointer();
        ctx.set_handled();
        ctx.request_render();
        true
    }

    pub(super) fn bar_hover(&mut self, ctx: &mut EventCtx<'_>, point: Point) {
        if self.block_bar.is_some_and(|(_, bar)| bar.dragging()) {
            return;
        }
        let next = self.block_under(point).filter(|(_, span)| span.track.contains(point)).map(|(start, span)| {
            let mut bar = self.bar_of(start);
            bar.hover(Some(point), &span);
            (start, bar)
        });
        if next.map(|(start, bar)| (start, bar.active())) != self.block_bar.map(|(start, bar)| (start, bar.active())) {
            ctx.request_render();
        }
        self.block_bar = next;
    }

    pub(super) fn bar_drag(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((start, bar)) = self.block_bar.filter(|(_, bar)| bar.dragging()) else { return false };
        if let Some(offset) = self.block_span(start).and_then(|span| bar.drag(point, &span)) {
            self.scroll_block(start, offset);
            ctx.request_render();
        }
        true
    }

    pub(super) fn bar_release(&mut self) {
        if let Some((_, bar)) = &mut self.block_bar {
            bar.release();
        }
    }

    pub(super) fn paint_block_bar(&self, painter: &mut Painter<'_>, start: usize) {
        if let Some(span) = self.block_span(start) {
            self.bar_of(start).paint(painter, &span);
        }
    }
}
