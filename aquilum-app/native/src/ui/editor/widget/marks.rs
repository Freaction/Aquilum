use super::*;
use markdown::{LIST_LEVEL, QUOTE_BAR};

const BULLET: f64 = 5.0;
const CALLOUT_RADIUS: f64 = 4.0;
const CALLOUT_ALPHA: f32 = 0.12;
const CHECKBOX: f64 = markdown::CHECKBOX as f64;
const CHECKBOX_RADIUS: f64 = 6.0;
const BLOCK_RADIUS: f64 = 12.0;
const RULE: f64 = 2.0;
const QUOTE_INSET: f64 = 2.0;

fn line_middle(layout: &Layout<BrushIndex>) -> f64 {
    layout.lines().next().map_or(0.0, |l| f64::from(l.metrics().min_coord + l.metrics().max_coord) / 2.0)
}

fn x_at(layout: &Layout<BrushIndex>, offset: usize) -> f64 {
    Cursor::from_byte_index(layout, offset, Affinity::Downstream).geometry(layout, 0.0).x0
}

impl TextEditor {
    pub(super) fn run(&self, i: usize, same: impl Fn(&Block) -> bool, starts: impl Fn(&Block) -> bool) -> (usize, usize) {
        let ps = self.doc.paragraphs();
        let mut a = i;
        while a > 0 && !starts(&ps[a].block) && same(&ps[a - 1].block) {
            a -= 1;
        }
        let mut b = i;
        while b + 1 < ps.len() && same(&ps[b + 1].block) && !starts(&ps[b + 1].block) {
            b += 1;
        }
        (a, b)
    }

    pub(super) fn paint_blocks(&self, painter: &mut Painter<'_>, first: usize, last: usize) {
        let tm = theme::current();
        let x = self.shift().x;
        let inner = (self.width - size::CARET_WIDTH * 2.0).max(0.0);
        let ps = self.doc.paragraphs();
        let mut i = first;
        while i <= last {
            let block = &ps[i].block;
            let (a, b) = match block {
                Block::Code { .. } => self.run(i, Block::is_code, |b| matches!(b, Block::Code { first: true, .. })),
                Block::Quote { .. } => self.run(i, Block::is_quote, |b| matches!(b, Block::Quote { callout: Some(_), .. })),
                _ => {
                    i += 1;
                    continue;
                }
            };
            let (top, bottom) = (ps[a].top, ps[b].top + ps[b].height);
            if block.is_code() {
                if ps[a].fold != Fold::Head {
                    painter.fill(RoundedRect::new(x, top, x + inner, bottom, BLOCK_RADIUS), tm.block_bg).draw();
                    let border = RoundedRect::new(x + 0.5, top + 0.5, x + inner - 0.5, bottom - 0.5, BLOCK_RADIUS - 0.5);
                    painter.stroke(border, &Stroke::new(size::BORDER_1), tm.block_border).draw();
                }
            } else if !matches!(ps[a].block, Block::Quote { callout: Some((_, Callout::Book)), .. }) {
                for (dx, from, to) in self.quote_bars(a, b) {
                    painter.fill(RoundedRect::new(x + dx, from + QUOTE_INSET, x + dx + QUOTE_BAR, to - QUOTE_INSET, QUOTE_BAR / 2.0), tm.bg_accent).draw();
                }
            }
            i = b + 1;
        }
    }

    fn quote_bars(&self, a: usize, b: usize) -> Vec<(f64, f64, f64)> {
        let ps = self.doc.paragraphs();
        let depth = |i: usize| match ps[i].block {
            Block::Quote { depth, .. } => usize::from(depth),
            _ => 0,
        };
        let ch = self.doc.markdown.as_ref().map_or(0.0, |fonts| f64::from(fonts.ch));
        let marker_x = |i: usize, level: usize| self.doc.text()[ps[i].range()].match_indices('>').nth(level - 1).map_or(0.0, |(at, _)| at as f64 * ch);
        let mut bars = Vec::new();
        for level in 1..=(a..=b).map(depth).max().unwrap_or(0) {
            let mut open = None;
            for i in a..=b + 1 {
                match (i <= b && depth(i) >= level, open) {
                    (true, None) => open = Some(i),
                    (false, Some(start)) => {
                        bars.push((marker_x(start, level), ps[start].top, ps[i - 1].top + ps[i - 1].height));
                        open = None;
                    }
                    _ => {}
                }
            }
        }
        bars
    }

    pub(super) fn paint_marks(&self, painter: &mut Painter<'_>, p: &Paragraph) {
        let tm = theme::current();
        let layout = p.layout();
        let origin = self.shift() + p.origin();
        let caret = p.decor().caret;
        let touching = |start: usize, end: usize| caret.is_some_and(|c| start <= c && c <= end);
        match &p.block {
            Block::Rule if caret.is_none() => {
                let inner = (self.width - size::CARET_WIDTH * 2.0).max(0.0);
                let (x, y) = (self.shift().x, p.top + p.height / 2.0);
                painter.fill(Rect::new(x, y - RULE / 2.0, x + inner, y + RULE / 2.0), tm.icon_quiet).draw();
            }
            Block::Item(item) => {
                let (marker, task) = (item.marker(), item.task());
                let middle = origin.y + line_middle(layout);
                if let Some((_, tone)) = item.callout().filter(|_| self.doc.markdown.as_ref().is_some_and(|fonts| fonts.callouts)) {
                    let inner = (self.width - size::CARET_WIDTH * 2.0).max(0.0);
                    let x = self.shift().x + LIST_LEVEL * item.level() as f64;
                    let shape = RoundedRect::new(x, p.top, self.shift().x + inner, p.top + p.height, CALLOUT_RADIUS);
                    painter.fill(shape, markdown::tone(&tm, tone).multiply_alpha(CALLOUT_ALPHA)).draw();
                }
                if !item.ordered && !touching(marker.start, marker.end - 1) {
                    let (x0, x1) = (x_at(layout, marker.start), x_at(layout, marker.start + 1));
                    painter.fill(Circle::new((origin.x + (x0 + x1) / 2.0, middle), BULLET / 2.0), tm.editor_list_marker_color).draw();
                }
                if let Some((range, checked)) = task
                    && !touching(range.start, range.end)
                {
                    let bounds = self.checkbox_rect(p, range.start);
                    let shape = RoundedRect::from_rect(bounds, CHECKBOX_RADIUS);
                    if checked {
                        painter.fill(shape, tm.bg_accent).draw();
                        let c = bounds.center();
                        let mut check = BezPath::new();
                        check.move_to((c.x - 4.5, c.y + 0.5));
                        check.line_to((c.x - 1.5, c.y + 3.5));
                        check.line_to((c.x + 4.5, c.y - 3.0));
                        painter.stroke(&check, &Stroke::new(2.0).with_caps(Cap::Round).with_join(Join::Round), tm.icon_on_accent).draw();
                    } else {
                        painter.fill(shape, tm.bg_surface_overlay).draw();
                    }
                }
            }
            _ => {}
        }
        markdown::paint_inline(painter, layout, p.decor(), origin);
    }

    pub(super) fn checkbox_rect(&self, p: &Paragraph, at: usize) -> Rect {
        let layout = p.layout();
        let origin = self.shift() + p.origin();
        let middle = origin.y + line_middle(layout);
        let x = origin.x + x_at(layout, at);
        Rect::new(x, middle - CHECKBOX / 2.0, x + CHECKBOX, middle + CHECKBOX / 2.0)
    }
}
