use masonry::kurbo::{BezPath, Point, Rect, RoundedRect};

use super::super::super::markdown::TEXT;
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, Label};
use super::{hue, number, split, unquote};
use crate::ui::tokens::size;

const WIDE: f64 = 640.0;
const TALL: f64 = 360.0;
const NODE: f64 = 12.0;
const GAP: f64 = 14.0;
const PASSES: usize = 8;

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (mut names, mut links): (Vec<String>, Vec<(usize, usize, f64)>) = (Vec::new(), Vec::new());
    for line in lines {
        let cells = split(line);
        let [a, b, v] = cells[..] else { continue };
        let Ok(value) = v.parse::<f64>() else { continue };
        let mut id = |name: &str| names.iter().position(|n| n == unquote(name)).unwrap_or_else(|| {
            names.push(unquote(name).to_owned());
            names.len() - 1
        });
        let (a, b) = (id(a), id(b));
        if value > 0.0 && a != b {
            links.push((a, b, value));
        }
    }
    let n = names.len();
    if links.is_empty() {
        return;
    }
    let mut col = vec![0usize; n];
    for _ in 0..n {
        for &(a, b, _) in &links {
            col[b] = col[b].max(col[a] + 1).min(n);
        }
    }
    let last = col.iter().copied().max().unwrap_or(0);
    (0..n).filter(|&v| links.iter().all(|l| l.0 != v)).for_each(|v| col[v] = last);
    let flow = |v: usize, side: fn(&(usize, usize, f64)) -> usize| links.iter().filter(|l| side(l) == v).map(|l| l.2).sum::<f64>();
    let size: Vec<f64> = (0..n).map(|v| flow(v, |l| l.0).max(flow(v, |l| l.1))).collect();
    let mut columns: Vec<Vec<usize>> = vec![Vec::new(); last + 1];
    (0..n).for_each(|v| columns[col[v]].push(v));
    let scale = columns.iter().map(|c| (TALL - GAP * c.len().saturating_sub(1) as f64) / c.iter().map(|&v| size[v]).sum::<f64>()).fold(f64::MAX, f64::min);
    let mut top = vec![0.0; n];
    let stack = |columns: &[Vec<usize>], top: &mut Vec<f64>| {
        for c in columns {
            let total = c.iter().map(|&v| size[v] * scale).sum::<f64>() + GAP * c.len().saturating_sub(1) as f64;
            let mut y = (TALL - total) / 2.0;
            for &v in c {
                top[v] = y;
                y += size[v] * scale + GAP;
            }
        }
    };
    stack(&columns, &mut top);
    for _ in 0..PASSES {
        for c in columns.iter_mut() {
            let centre = |v: usize| {
                let near = links.iter().filter_map(|&(a, b, w)| if a == v { Some((b, w)) } else if b == v { Some((a, w)) } else { None });
                let (sum, weight) = near.fold((0.0, 0.0), |(s, t), (u, w)| (s + (top[u] + size[u] * scale / 2.0) * w, t + w));
                if weight > 0.0 { sum / weight } else { top[v] }
            };
            let mut keyed: Vec<(f64, usize)> = c.iter().map(|&v| (centre(v), v)).collect();
            keyed.sort_by(|a, b| a.0.total_cmp(&b.0));
            *c = keyed.into_iter().map(|(_, v)| v).collect();
        }
        stack(&columns, &mut top);
    }
    let step = (WIDE - NODE) / last.max(1) as f64;
    let x = |v: usize| col[v] as f64 * step;
    let (mut out, mut into) = (top.clone(), top.clone());
    let mut order: Vec<usize> = (0..links.len()).collect();
    order.sort_by(|&i, &j| top[links[i].1].total_cmp(&top[links[j].1]).then(top[links[i].0].total_cmp(&top[links[j].0])));
    let (outflow, inflow): (Vec<f64>, Vec<f64>) = (0..n).map(|v| (flow(v, |l| l.0), flow(v, |l| l.1))).unzip();
    for i in order {
        let (a, b, w) = links[i];
        let (from, into_b) = (w * scale * size[a] / outflow[a], w * scale * size[b] / inflow[b]);
        let (x0, x1) = (x(a) + NODE, x(b));
        let (ya, yb) = (out[a], into[b]);
        let mid = (x0 + x1) / 2.0;
        let mut band = BezPath::new();
        band.move_to((x0, ya));
        band.curve_to((mid, ya), (mid, yb), (x1, yb));
        band.line_to((x1, yb + into_b));
        band.curve_to((mid, yb + into_b), (mid, ya + from), (x0, ya + from));
        band.close_path();
        canvas.wash(band, Tone::Panel(hue(a)));
        (out[a], into[b]) = (ya + from, yb + into_b);
    }
    for v in 0..n {
        let r = Rect::new(x(v), top[v], x(v) + NODE, top[v] + (size[v] * scale).max(1.0));
        canvas.shape(RoundedRect::from_rect(r, size::RADIUS_SM / 2.0), Some(Tone::Ink(hue(v))), None);
        let text = format!("{}  {}", names[v], number(size[v]));
        let label = Label::new(Point::new(0.0, r.center().y), text, FONT - 1.0, TEXT);
        canvas.label(if col[v] == last && last > 0 { Label { at: Point::new(r.x0 - size::GAP_SM, r.center().y), ..label.beside(-1.0) } } else { Label { at: Point::new(r.x1 + size::GAP_SM, r.center().y), ..label.start(0.0) } });
    }
}
