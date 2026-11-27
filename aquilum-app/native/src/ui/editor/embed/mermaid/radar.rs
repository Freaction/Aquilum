use std::f64::consts::{FRAC_PI_2, TAU};

use masonry::kurbo::{Circle, Line, Point, Rect, RoundedRect, Vec2};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINE, LINK, Label, Pen, width};
use super::shape::polygon;
use super::{hue, number, split, unquote};
use crate::ui::tokens::size;

const RADIUS: f64 = 140.0;
const DOT: f64 = 3.0;
const MARKER: f64 = 10.0;

fn named(item: &str) -> (&str, &str) {
    let item = item.trim();
    match (item.find('['), item.rfind(']')) {
        (Some(a), Some(b)) if b > a => (item[..a].trim(), unquote(&item[a + 1..b])),
        _ => (item, unquote(item)),
    }
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (mut title, mut axes, mut curves) = (None, Vec::<(&str, &str)>::new(), Vec::new());
    let (mut low, mut high, mut ticks, mut round) = (None, None, 5usize, true);
    for line in lines.iter().map(|l| l.trim()) {
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        match word {
            "title" => title = Some(unquote(rest)),
            "axis" => axes.extend(split(rest).into_iter().map(named)),
            "curve" => {
                let (head, body) = rest.split_once('{').unwrap_or((rest, ""));
                curves.push((named(head).1, split(body.trim_end_matches('}')).into_iter().map(|v| v.split_once(':').map_or((None, v), |(k, v)| (Some(k.trim()), v))).collect::<Vec<_>>()));
            }
            "max" => high = rest.trim().parse::<f64>().ok(),
            "min" => low = rest.trim().parse::<f64>().ok(),
            "ticks" => ticks = rest.trim().parse::<usize>().unwrap_or(5).max(1),
            "graticule" => round = rest.trim() != "polygon",
            _ => {}
        }
    }
    if axes.len() < 3 {
        return;
    }
    let values: Vec<Vec<f64>> = curves.iter().map(|(_, items)| axes.iter().enumerate().map(|(i, (id, _))| items.iter().find(|(k, _)| *k == Some(*id)).or_else(|| items.get(i).filter(|(k, _)| k.is_none())).and_then(|(_, v)| v.trim().parse().ok()).unwrap_or(0.0)).collect()).collect();
    let low = low.unwrap_or(0.0);
    let high = high.unwrap_or_else(|| values.iter().flatten().copied().fold(low + 1.0, f64::max));
    let n = axes.len();
    let dir = |i: usize| Vec2::from_angle(-FRAC_PI_2 + TAU * i as f64 / n as f64);
    let at = |i: usize, v: f64| Point::ORIGIN + dir(i) * RADIUS * ((v - low) / (high - low)).clamp(0.0, 1.0);
    let grid = Pen::border(Tone::Border);
    for t in 1..=ticks {
        let r = RADIUS * t as f64 / ticks as f64;
        if round {
            canvas.shape(Circle::new(Point::ORIGIN, r), None, Some(grid));
        } else {
            canvas.shape(polygon(&(0..n).map(|i| Point::ORIGIN + dir(i) * r).collect::<Vec<_>>(), true), None, Some(grid));
        }
        canvas.label(Label::new(Point::new(size::GAP_XS, -r), number(low + (high - low) * t as f64 / ticks as f64), FONT - 4.0, MUTED).start(0.0));
    }
    for (i, (_, name)) in axes.iter().enumerate() {
        canvas.shape(Line::new(Point::ORIGIN, Point::ORIGIN + dir(i) * RADIUS), None, Some(grid));
        let d = dir(i);
        let label = Label::new(Point::ORIGIN + d * (RADIUS + size::GAP_LG + LINE / 2.0 * d.y.abs()), *name, FONT - 1.0, TEXT);
        canvas.label(if d.x.abs() > 0.3 { label.beside(d.x) } else { label });
    }
    for (k, row) in values.iter().enumerate() {
        let points: Vec<Point> = row.iter().enumerate().map(|(i, &v)| at(i, v)).collect();
        canvas.wash(polygon(&points, true), Tone::Panel(hue(k)));
        canvas.shape(polygon(&points, true), None, Some(Pen::new(Tone::Ink(hue(k)), LINK)));
        points.iter().for_each(|&p| canvas.shape(Circle::new(p, DOT), Some(Tone::Ink(hue(k))), None));
    }
    let mut x = -RADIUS;
    let y = RADIUS + LINE * 2.5;
    for (k, (name, _)) in curves.iter().enumerate() {
        canvas.shape(RoundedRect::from_rect(Rect::from_center_size((x + MARKER / 2.0, y), (MARKER, MARKER)), size::RADIUS_SM), Some(Tone::Ink(hue(k))), None);
        canvas.label(Label::new(Point::new(x + MARKER + size::GAP_SM, y), *name, FONT - 1.0, TEXT).start(0.0));
        x += MARKER + size::GAP_SM + width(name, FONT - 1.0) + size::GAP_XL;
    }
    if let Some(title) = title {
        canvas.label(Label::new(Point::new(0.0, -RADIUS - LINE * 2.0), title, FONT + 1.0, TEXT).bold());
    }
}
