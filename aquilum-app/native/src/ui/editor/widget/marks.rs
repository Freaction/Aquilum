use super::*;

const BULLET: f64 = 5.0;
const CALLOUT_RADIUS: f64 = 4.0;
const CALLOUT_ALPHA: f32 = 0.12;
const LIST_LEVEL: f64 = 24.0;
const TAG_PAD_X: f64 = 4.0;
const TAG_HEIGHT: f64 = 24.0;
const TAG_ALPHA: f32 = 0.1;
const CHECKBOX: f64 = 20.0;
const CHECKBOX_RADIUS: f64 = 6.0;
pub(super) const TASK_BOX: usize = 3;
const BLOCK_RADIUS: f64 = 12.0;
const RULE: f64 = 2.0;
const QUOTE_BAR: f64 = 3.0;
const QUOTE_INSET: f64 = 2.0;
const CODE_PAD_X: f64 = 4.0;
const CODE_PAD_Y: f64 = 2.0;
const CODE_RADIUS: f64 = 2.0;

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
                painter.fill(RoundedRect::new(x, top, x + inner, bottom, BLOCK_RADIUS), tm.block_bg).draw();
                let border = RoundedRect::new(x + 0.5, top + 0.5, x + inner - 0.5, bottom - 0.5, BLOCK_RADIUS - 0.5);
                painter.stroke(border, &Stroke::new(size::BORDER_1), tm.block_border).draw();
            } else if !matches!(ps[a].block, Block::Quote { callout: Some((_, Callout::Book)), .. }) {
                painter.fill(RoundedRect::new(x, top + QUOTE_INSET, x + QUOTE_BAR, bottom - QUOTE_INSET, QUOTE_BAR / 2.0), tm.bg_accent).draw();
            }
            i = b + 1;
        }
    }

    pub(super) fn paint_marks(&self, painter: &mut Painter<'_>, p: &Paragraph) {
        let tm = theme::current();
        let layout = p.layout();
        let origin = self.shift() + p.origin();
        let caret = p.decor.caret;
        let touching = |start: usize, end: usize| caret.is_some_and(|c| start <= c && c <= end);
        match &p.block {
            Block::Rule if caret.is_none() => {
                let inner = (self.width - size::CARET_WIDTH * 2.0).max(0.0);
                let (x, y) = (self.shift().x, p.top + p.height / 2.0);
                painter.fill(Rect::new(x, y - RULE / 2.0, x + inner, y + RULE / 2.0), tm.icon_quiet).draw();
            }
            Block::Item { level, marker, ordered, task, callout, .. } => {
                let middle = origin.y + line_middle(layout);
                if let Some((_, tone)) = callout {
                    let inner = (self.width - size::CARET_WIDTH * 2.0).max(0.0);
                    let x = self.shift().x + LIST_LEVEL * *level as f64;
                    let shape = RoundedRect::new(x, p.top, self.shift().x + inner, p.top + p.height, CALLOUT_RADIUS);
                    painter.fill(shape, markdown::tone(tm, *tone).multiply_alpha(CALLOUT_ALPHA)).draw();
                }
                if !*ordered && !touching(marker.start, marker.end - 1) {
                    let (x0, x1) = (x_at(layout, marker.start), x_at(layout, marker.start + 1));
                    painter.fill(Circle::new((origin.x + (x0 + x1) / 2.0, middle), BULLET / 2.0), tm.editor_list_marker_color).draw();
                }
                if let Some((range, checked)) = task
                    && !touching(range.start, range.start + TASK_BOX)
                {
                    let bounds = self.checkbox_rect(p, range.start);
                    let shape = RoundedRect::from_rect(bounds, CHECKBOX_RADIUS);
                    if *checked {
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
        for tag in &p.decor.tags {
            let (x0, x1) = (x_at(layout, tag.start), x_at(layout, tag.end));
            let middle = origin.y + {
                let y = Cursor::from_byte_index(layout, tag.start, Affinity::Downstream).geometry(layout, 0.0);
                (y.y0 + y.y1) / 2.0
            };
            let shape = RoundedRect::new(origin.x + x0 - TAG_PAD_X, middle - TAG_HEIGHT / 2.0, origin.x + x1 + TAG_PAD_X, middle + TAG_HEIGHT / 2.0, TAG_HEIGHT / 2.0);
            painter.fill(shape, tm.bg_accent.multiply_alpha(TAG_ALPHA)).draw();
        }
        for code in &p.decor.codes {
            let selection = Selection::new(Cursor::from_byte_index(layout, code.start, Affinity::Downstream), Cursor::from_byte_index(layout, code.end, Affinity::Upstream));
            for (bounds, _) in selection.geometry(layout) {
                let middle = ((bounds.y0 + bounds.y1) / 2.0) as f32;
                let Some(m) = layout.lines().map(|l| *l.metrics()).find(|m| m.min_coord <= middle && middle < m.max_coord) else { continue };
                let (top, bottom) = (f64::from(m.baseline - m.ascent) - CODE_PAD_Y, f64::from(m.baseline + m.descent) + CODE_PAD_Y);
                let shape = RoundedRect::new(bounds.x0 - CODE_PAD_X, top, bounds.x1 + CODE_PAD_X, bottom, CODE_RADIUS) + origin;
                painter.fill(shape, tm.editor_code_bg).draw();
            }
        }
    }

    pub(super) fn checkbox_rect(&self, p: &Paragraph, at: usize) -> Rect {
        let layout = p.layout();
        let origin = self.shift() + p.origin();
        let middle = origin.y + line_middle(layout);
        let x = origin.x + x_at(layout, at);
        Rect::new(x, middle - CHECKBOX / 2.0, x + CHECKBOX, middle + CHECKBOX / 2.0)
    }
}
