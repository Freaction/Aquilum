use std::ops::RangeInclusive;

use super::*;
use crate::ui::editor::embed::View;
use crate::ui::editor::highlight::Lang;
use crate::ui::editor::embed::panel::{self, PAD};
use crate::ui::scroll::bar::Span;
use masonry::core::render_text;
use masonry::peniko::Brush;

pub(super) const COPIED: std::time::Duration = std::time::Duration::from_millis(1500);

pub(super) struct Code {
    pub(super) first: usize,
    last: usize,
    header: bool,
    digits: usize,
    body: RangeInclusive<usize>,
    panel: Rect,
    area: Rect,
}

impl TextEditor {
    pub(super) fn code(&self, i: usize, fonts: &Fonts) -> Code {
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

    pub(super) fn code_span(&self, code: &Code) -> Option<Span> {
        if code.body.is_empty() {
            return None;
        }
        let content = self.shown(&code.body).filter_map(|j| self.doc.paragraph(j).laid()).map(|layout| f64::from(layout.width())).fold(0.0, f64::max);
        let track = Rect::new(code.area.x0, code.area.y1 - size::SCROLLBAR_WIDTH, code.area.x1, code.area.y1);
        let span = Span { track, viewport: track.width(), content, offset: self.doc.scroll_of(self.doc.paragraph(*code.body.start()).start) };
        (span.max() > 0.0).then(|| Span { offset: span.offset.min(span.max()), ..span })
    }

    pub(super) fn code_at(&self, point: Point) -> Option<Code> {
        let fonts = self.doc.markdown.as_ref()?;
        let code = self.code(self.index_at(point).filter(|&i| self.doc.block(i).is_code())?, fonts);
        (self.doc.paragraph(code.first).fold != Fold::Head && code.panel.contains(point)).then_some(code)
    }

    fn copy_at(&self, point: Point) -> Option<Code> {
        let header = panel::header(self.doc.markdown.as_ref()?.line);
        self.code_at(point).filter(|code| code.header && panel::button(code.panel, header).contains(point))
    }

    pub(super) fn scroll_code(&mut self, code: &Code, x: f64) {
        let (start, end) = (self.doc.paragraph(*code.body.start()).start, self.doc.paragraph(*code.body.end()).end());
        self.doc.scrolls.retain(|(from, _, _)| *from != start);
        if x > 0.0 {
            self.doc.scrolls.push((start, end, x));
        }
    }

    pub(super) fn copy_press(&mut self, ctx: &mut EventCtx<'_>, point: Point) -> bool {
        let Some(code) = self.copy_at(point) else { return false };
        let body: Vec<&str> = code.body.clone().map(|j| &self.doc.line(j)[self.doc.frame(j).lead..]).collect();
        ctx.set_clipboard(body.join("\n"));
        self.copied = Some((self.doc.paragraph(code.first).start, std::time::Instant::now()));
        ctx.request_anim_frame();
        ctx.set_handled();
        ctx.request_render();
        true
    }

    pub(super) fn copy_hover(&self, point: Point) -> Option<usize> {
        self.copy_at(point).map(|code| self.doc.paragraph(code.first).start)
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
        let button = (!self.doc.print).then_some((crate::ui::icons::COPY, hovered, done));
        let header = code.header.then(|| panel::Header { height: panel::header(fonts.line), icon: None, button });
        panel::paint(painter, code.panel, panel::Tone::Surface, header);
        self.paint_block_bar(painter, start);
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
