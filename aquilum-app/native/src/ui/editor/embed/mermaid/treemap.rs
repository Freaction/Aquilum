use masonry::kurbo::{Point, Rect, RoundedRect};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINE, Label, Pen};
use super::{hue, number, unquote};
use crate::ui::tokens::size;

const WIDE: f64 = 600.0;
const TALL: f64 = 380.0;
const HEAD: f64 = LINE + 6.0;
const PAD: f64 = 4.0;
const GAP: f64 = 2.0;

struct Item<'a> {
    name: &'a str,
    value: f64,
    kids: Vec<usize>,
    top: usize,
}

fn worst(row: &[f64], side: f64) -> f64 {
    let sum: f64 = row.iter().sum();
    let (lo, hi) = row.iter().fold((f64::MAX, 0.0f64), |(lo, hi), &a| (lo.min(a), hi.max(a)));
    (side * side * hi / (sum * sum)).max(sum * sum / (side * side * lo))
}

fn squarify(values: &[f64], mut rest: Rect) -> Vec<Rect> {
    let scale = rest.area() / values.iter().sum::<f64>();
    let areas: Vec<f64> = values.iter().map(|v| v * scale).collect();
    let (mut out, mut i) = (Vec::new(), 0);
    while i < areas.len() {
        let side = rest.width().min(rest.height());
        let mut j = i + 1;
        while j < areas.len() && worst(&areas[i..=j], side) <= worst(&areas[i..j], side) {
            j += 1;
        }
        let sum: f64 = areas[i..j].iter().sum();
        let across = rest.width() >= rest.height();
        let thick = sum / if across { rest.height() } else { rest.width() };
        let mut at = if across { rest.y0 } else { rest.x0 };
        for a in &areas[i..j] {
            let long = a / thick;
            out.push(if across { Rect::new(rest.x0, at, rest.x0 + thick, at + long) } else { Rect::new(at, rest.y0, at + long, rest.y0 + thick) });
            at += long;
        }
        if across { rest.x0 += thick } else { rest.y0 += thick }
        i = j;
    }
    out
}

fn place(items: &[Item], kids: &[usize], rect: Rect, out: &mut Vec<(usize, Rect)>) {
    let mut kids: Vec<usize> = kids.iter().copied().filter(|&k| items[k].value > 0.0).collect();
    kids.sort_by(|a, b| items[*b].value.total_cmp(&items[*a].value));
    if kids.is_empty() || rect.width() < 1.0 || rect.height() < 1.0 {
        return;
    }
    for (&k, r) in kids.iter().zip(squarify(&kids.iter().map(|&k| items[k].value).collect::<Vec<_>>(), rect)) {
        out.push((k, r));
        if !items[k].kids.is_empty() {
            place(items, &items[k].kids, Rect::new(r.x0 + PAD, r.y0 + HEAD, r.x1 - PAD, r.y1 - PAD), out);
        }
    }
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let mut items: Vec<Item> = Vec::new();
    let (mut roots, mut stack) = (Vec::new(), Vec::<(usize, usize)>::new());
    for line in lines {
        let indent = line.len() - line.trim_start().len();
        let text = line.trim().split(":::").next().unwrap_or_default();
        let (name, value) = match text.rsplit_once(':') {
            Some((name, value)) => (unquote(name), value.trim().parse().unwrap_or(0.0)),
            None => (unquote(text), 0.0),
        };
        while stack.last().is_some_and(|&(i, _)| i >= indent) {
            stack.pop();
        }
        let k = items.len();
        let top = stack.first().map_or(roots.len(), |&(_, root)| items[root].top);
        items.push(Item { name, value, kids: Vec::new(), top });
        match stack.last() {
            Some(&(_, parent)) => items[parent].kids.push(k),
            None => roots.push(k),
        }
        stack.push((indent, k));
    }
    for k in (0..items.len()).rev() {
        if !items[k].kids.is_empty() {
            let sum = items[k].kids.iter().map(|&c| items[c].value).sum();
            items[k].value = sum;
        }
    }
    let mut placed = Vec::new();
    place(&items, &roots, Rect::new(0.0, 0.0, WIDE, TALL), &mut placed);
    for (k, r) in placed {
        let (item, r) = (&items[k], r.inflate(-GAP, -GAP));
        let tint = hue(item.top);
        if item.kids.is_empty() {
            canvas.shape(RoundedRect::from_rect(r, size::RADIUS_SM), Some(Tone::Panel(tint)), None);
            if r.width() > 40.0 && r.height() > LINE * 1.8 {
                let at = r.center();
                canvas.label(Label { width: r.width() - size::GAP_MD * 2.0, ..Label::new(Point::new(at.x, at.y - LINE / 2.0), item.name, FONT - 1.0, TEXT).bold() });
                canvas.label(Label::new(Point::new(at.x, at.y + LINE / 2.0), number(item.value), FONT - 2.0, MUTED));
            }
        } else {
            canvas.shape(RoundedRect::from_rect(r, size::RADIUS_MD), None, Some(Pen::border(Tone::Ink(tint))));
            canvas.label(Label::new(Point::new(r.x0 + size::GAP_MD, r.y0 + HEAD / 2.0), item.name, FONT - 1.0, TEXT).bold().start(0.0));
        }
    }
}
