use masonry::kurbo::{Point, Rect, RoundedRect};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINE, Label};
use super::{hue, titled, unquote};
use crate::ui::tokens::size;

const BITS: usize = 32;
const CELL: f64 = 20.0;
const ROW: f64 = 48.0;
const GAP: f64 = 2.0;
const SMALL: f64 = FONT - 4.0;

fn field(range: &str, next: usize) -> Option<(usize, usize)> {
    let range = range.trim();
    if let Some(count) = range.strip_prefix('+') {
        let count: usize = count.trim().parse().ok()?;
        return (count > 0).then(|| (next, next + count - 1));
    }
    let (a, b) = range.split_once('-').unwrap_or((range, range));
    let (a, b): (usize, usize) = (a.trim().parse().ok()?, b.trim().parse().ok()?);
    (b >= a).then_some((a, b))
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (title, body) = titled(lines);
    let (mut fields, mut next) = (Vec::new(), 0);
    for (range, name) in body.iter().filter_map(|l| l.split_once(':')) {
        let Some((start, end)) = field(range, next) else { continue };
        fields.push((start, end, unquote(name)));
        next = end + 1;
    }
    if fields.is_empty() {
        return;
    }
    let top = if title.is_some() { LINE * 2.0 } else { 0.0 };
    if let Some(title) = title {
        canvas.label(Label::new(Point::new(BITS as f64 * CELL / 2.0, LINE / 2.0), title, FONT + 1.0, TEXT).bold());
    }
    for (k, &(start, end, name)) in fields.iter().enumerate() {
        let mut bit = start;
        while bit <= end {
            let row = bit / BITS;
            let last = end.min(row * BITS + BITS - 1);
            let y0 = top + row as f64 * ROW;
            let r = Rect::new((bit % BITS) as f64 * CELL, y0, (last % BITS + 1) as f64 * CELL, y0 + ROW).inflate(-GAP, -GAP);
            canvas.shape(RoundedRect::from_rect(r, size::RADIUS_SM), Some(Tone::Panel(hue(k))), None);
            canvas.label(Label { width: r.width() - GAP * 2.0, ..Label::new(Point::new(r.center().x, r.center().y + SMALL / 2.0), name, FONT - 1.0, TEXT) });
            let corner = r.y0 + SMALL * 0.9;
            canvas.label(Label::new(Point::new(r.x0 + size::GAP_XS, corner), bit.to_string(), SMALL, MUTED).start(0.0));
            if last != bit {
                canvas.label(Label::new(Point::new(r.x1 - size::GAP_XS, corner), last.to_string(), SMALL, MUTED).beside(-1.0));
            }
            bit = last + 1;
        }
    }
}
