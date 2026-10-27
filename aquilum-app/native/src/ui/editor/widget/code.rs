use std::ops::RangeInclusive;

use super::*;
use crate::ui::editor::embed::View;
use crate::ui::editor::highlight::Lang;
use crate::ui::editor::embed::panel::{self, PAD};
use crate::ui::scroll::bar::{Scrollbar, Span};
use masonry::core::render_text;
use masonry::peniko::Brush;

pub(super) const COPIED: std::time::Duration = std::time::Duration::from_millis(1500);

struct Code {
    first: usize,
    last: usize,
    header: bool,
    digits: usize,
    body: RangeInclusive<usize>,
    panel: Rect,
    area: Rect,
}

impl TextEditor {
    fn code(&self, i: usize, fonts: &Fonts) -> Code {
        let block = |j: usize| self.doc.block(j);
        let (first, last) = self.doc.code_run(i);
        let fenced = |j: usize| matches!(block(j), Block::Code { fence: true, .. });
        let body = first + usize::from(fenced(first))..=last - usize::from(last > first && fenced(last));
        let math = matches!(block(first), Block::Code { lang: Some(Lang::Math), .. });
        let header = !math && matches!(block(first), Block::Code { first: true, fence: true, .. });
        let frame = self.doc.view(first);
        let x = self.shift().x + frame.x(self.doc.line(first), fonts);
        let bottom = self.doc.bottom(last) - f64::from(self.doc.view(last).closes) * PAD;
        let panel = Rect::new(x, self.doc.top(first), self.shift().x + self.inner_width() - frame.right(), bottom);
        let digits = if body.is_empty() || math { 0 } else { panel::digits(body.end() - body.start() + 1) };
        let area = Rect::new(panel.x0 + PAD + panel::gutter(digits, fonts.ch), panel.y0, panel.x1, panel.y1);
        Code { first, last, header, digits, body, panel, area }
    }

    fn shown(&self, body: &RangeInclusive<usize>) -> RangeInclusive<usize> {
        self.doc.at_y(self.painted.0).max(*body.start())..=self.doc.at_y(self.painted.1).min(*body.end())
    }

    fn code_span(&self, code: &Code) -> Option<Span> {
        if code.body.is_empty() {
            return None;
        }
        let content = self.shown(&code.body).filter_map(|j| self.doc.paragraph(j).laid()).map(|layout| f64::from(layout.width())).fold(0.0, f64::max);
        let track = Rect::new(code.area.x0, code.area.y1 - size::SCROLLBAR_WIDTH, code.area.x1, code.area.y1);
        let span = Span { track, viewport: track.width(), content, offset: self.doc.scroll_of(self.doc.paragraph(*code.body.start()).start) };
        (span.max() > 0.0).then(|| Span { offset: span.offset.min(span.max()), ..span })
    }

    fn code_at(&self, point: Point) -> Option<(Code, f64)> {
        let fonts = self.doc.markdown.as_ref()?;
        let code = self.code(self.index_at(point).filter(|&i| self.doc.block(i).is_code())?, fonts);
        code.panel.contains(point).then_some((code, panel::header(fonts.line)))
    }

    fn copy_button(&self, code: &Code, header: f64, point: Point) -> bool {
        code.header && panel::button(code.panel, header).contains(point)
    }

    fn code_bar(&self, code: &Code) -> Scrollbar {
        self.code_bar.filter(|(start, _)| *start == self.doc.paragraph(code.first).start).map_or(Scrollbar::new(Axis::Horizontal), |(_, bar)| bar)
    }

    fn scroll_code(&mut self, code: &Code, x: f64) {
        let (start, end) = (self.doc.paragraph(*code.body.start()).start, self.doc.paragraph(*code.body.end()).end());
        self.doc.scrolls.retain(|(from, _, _)| *from != start);
        if x > 0.0 {
            self.doc.scrolls.push((start, end, x));
        }
    }

    pub(super) fn code_scroll(&mut self, ctx: &mut EventCtx<'_>, point: Point, dx: f64) -> bool {
        let Some((code, _)) = self.code_at(point) else { return false };
        let Some(span) = self.code_span(&code) else { return false };
        self.scroll_code(&code, (span.offset + dx).clamp(0.0, span.max()));
        ctx.request_render();
        true
    }

    pub(super) fn code_press(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((code, header)) = self.code_at(point) else { return false };
        if self.copy_button(&code, header, point) {
            let body: Vec<&str> = code.body.clone().map(|j| &self.doc.line(j)[self.doc.frame(j).lead..]).collect();
            ctx.set_clipboard(body.join("\n"));
            self.copied = Some((self.doc.paragraph(code.first).start, std::time::Instant::now()));
            ctx.request_anim_frame();
        } else {
            let Some(span) = self.code_span(&code) else { return false };
            let mut bar = self.code_bar(&code);
            let Some(offset) = bar.press(point, &span) else { return false };
            self.code_bar = Some((self.doc.paragraph(code.first).start, bar));
            self.scroll_code(&code, offset);
            ctx.capture_pointer();
        }
        ctx.set_handled();
        ctx.request_render();
        true
    }

    pub(super) fn code_hover(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> Option<usize> {
        let found = self.code_at(point);
        let button = found.as_ref().filter(|(code, header)| self.copy_button(code, *header, point)).map(|(code, _)| self.doc.paragraph(code.first).start);
        if self.code_bar.is_some_and(|(_, bar)| bar.dragging()) {
            return button;
        }
        let next = found.and_then(|(code, _)| Some((self.code_span(&code)?, code))).filter(|(span, _)| span.track.contains(point)).map(|(span, code)| {
            let mut bar = self.code_bar(&code);
            bar.hover(Some(point), &span);
            (self.doc.paragraph(code.first).start, bar)
        });
        if next.map(|(start, bar)| (start, bar.active())) != self.code_bar.map(|(start, bar)| (start, bar.active())) {
            ctx.request_render();
        }
        self.code_bar = next;
        button
    }

    pub(super) fn code_drag(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some((start, bar)) = self.code_bar.filter(|(_, bar)| bar.dragging()) else { return false };
        let Some(fonts) = self.doc.markdown.as_ref() else { return false };
        let code = self.code(self.doc.at_offset(start), fonts);
        if let Some(offset) = self.code_span(&code).and_then(|span| bar.drag(point, &span)) {
            self.scroll_code(&code, offset);
            ctx.request_render();
        }
        true
    }

    pub(super) fn code_release(&mut self) {
        if let Some((_, bar)) = &mut self.code_bar {
            bar.release();
        }
    }

    pub(super) fn follow_code(&mut self) {
        let Some(fonts) = self.doc.markdown.as_ref() else { return };
        let i = self.doc.at_offset(self.focus);
        if !self.laid_out() || !self.doc.paragraph(i).decor().nowrap {
            return;
        }
        let code = self.code(i, fonts);
        let caret = self.caret().x0;
        let shift = if caret < code.area.x0 { caret - code.area.x0 } else { (caret + size::CARET_WIDTH - code.area.x1).max(0.0) };
        if shift != 0.0 {
            let offset = self.doc.scroll_of(self.doc.paragraph(i).start);
            self.scroll_code(&code, (offset + shift).max(0.0));
        }
    }

    pub(super) fn paint_codes(&self, painter: &mut Painter<'_>, first: usize, last: usize, fonts: &Fonts, palette: &[Brush]) -> Vec<(RangeInclusive<usize>, Rect)> {
        let mut clips = Vec::new();
        let mut i = first;
        while i <= last {
            if self.doc.block(i).is_code() {
                let code = self.code(i, fonts);
                i = code.last;
                if self.doc.paragraph(code.first).fold != Fold::Head {
                    clips.push(self.paint_code(painter, code, fonts, palette));
                }
            }
            i += 1;
        }
        clips
    }

    fn paint_code(&self, painter: &mut Painter<'_>, code: Code, fonts: &Fonts, palette: &[Brush]) -> (RangeInclusive<usize>, Rect) {
        let start = self.doc.paragraph(code.first).start;
        let (hovered, done) = (self.hovered_link == Some((start, 0)), self.copied.is_some_and(|(at, _)| at == start));
        let header = code.header.then(|| panel::Header { height: panel::header(fonts.line), icon: None, button: Some((crate::ui::icons::COPY, hovered, done)) });
        panel::paint(painter, code.panel, panel::Tone::Surface, header);
        if let Some(span) = self.code_span(&code) {
            self.code_bar(&code).paint(painter, &span);
        }
        if let (Fold::Below, Some(View::Drawing(preview))) = (self.doc.paragraph(code.last).fold, self.doc.paragraph(code.first).embed.map(|e| &e.view)) {
            preview.paint(painter, self.embed_x(code.first), code.panel.y1 - preview.height, 0.0, palette, None, None);
        }
        let baseline = |layout: &Layout<BrushIndex>| layout.lines().next().map_or(0.0, |line| f64::from(line.metrics().baseline));
        for j in self.shown(&code.body).filter(|_| code.digits > 0) {
            let y = self.shift().y + self.doc.origin(j).y + self.doc.paragraph(j).laid().map_or(0.0, baseline) - baseline(&fonts.digits[0]);
            let (mut number, mut x) = (j - code.body.start() + 1, code.area.x0 - PAD);
            while number > 0 {
                x -= f64::from(fonts.ch);
                render_text(painter, Affine::translate((x, y)), &fonts.digits[number % 10], palette, true);
                number /= 10;
            }
        }
        (code.body, code.area)
    }
}
