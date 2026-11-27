use masonry::kurbo::{BezPath, Circle, Line, Point, Rect, RoundedRect, RoundedRectRadii};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINE, Label, Pen};
use super::{hue, number, titled, unquote};
use crate::ui::tokens::size;

const HEIGHT: f64 = 240.0;
const STEP: f64 = 56.0;
const ROW: f64 = 36.0;
const WIDE: f64 = 360.0;
const TICKS: f64 = 5.0;
const BARS: f64 = 0.6;
const DOT: f64 = 3.5;
const SMALL: f64 = FONT - 2.0;

struct Series {
    bar: bool,
    values: Vec<f64>,
}

struct Axis {
    title: String,
    names: Vec<String>,
    range: Option<(f64, f64)>,
}

fn axis(rest: &str) -> Axis {
    let rest = rest.trim();
    if let (Some(a), Some(b)) = (rest.find('['), rest.rfind(']')) {
        let names = rest[a + 1..b].split(',').map(|n| unquote(n).to_owned()).filter(|n| !n.is_empty()).collect();
        return Axis { title: unquote(&rest[..a]).to_owned(), names, range: None };
    }
    let (head, high) = rest.split_once("-->").unwrap_or((rest, ""));
    let low = head.rsplit(char::is_whitespace).next().unwrap_or_default();
    let range = low.parse().ok().zip(high.trim().parse().ok());
    let title = if range.is_some() { &head[..head.len() - low.len()] } else { rest };
    Axis { title: unquote(title).to_owned(), names: Vec::new(), range }
}

fn values(rest: &str) -> Vec<f64> {
    let inside = rest.find('[').zip(rest.rfind(']')).map_or("", |(a, b)| &rest[a + 1..b]);
    inside.split(',').filter_map(|v| v.trim().parse().ok()).collect()
}

fn step(span: f64) -> f64 {
    let rough = span / TICKS;
    let power = 10f64.powf(rough.log10().floor());
    [1.0, 2.0, 2.5, 5.0, 10.0].iter().map(|m| m * power).find(|s| *s >= rough).unwrap_or(rough)
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    chart(lines, false, canvas);
}

pub(super) fn across(lines: &[&str], canvas: &mut Canvas) {
    chart(lines, true, canvas);
}

fn chart(lines: &[&str], across: bool, canvas: &mut Canvas) {
    let (title, body) = titled(lines);
    let (mut x, mut y, mut series) = (None, None, Vec::new());
    for line in body {
        if let Some(rest) = line.strip_prefix("x-axis") {
            x = Some(axis(rest));
        } else if let Some(rest) = line.strip_prefix("y-axis") {
            y = Some(axis(rest));
        } else if let Some(rest) = line.strip_prefix("bar") {
            series.push(Series { bar: true, values: values(rest) });
        } else if let Some(rest) = line.strip_prefix("line") {
            series.push(Series { bar: false, values: values(rest) });
        }
    }
    let count = series.iter().map(|s| s.values.len()).max().unwrap_or(0);
    if count == 0 {
        return;
    }
    let all = series.iter().flat_map(|s| s.values.iter().copied());
    let (low, high) = y.as_ref().and_then(|a| a.range).unwrap_or_else(|| all.fold((0.0f64, f64::MIN), |(lo, hi), v| (lo.min(v), hi.max(v))));
    let tick = step((high - low).max(1e-9));
    let (low, high) = ((low / tick).floor() * tick, ((high / tick).ceil() * tick).max(low + tick));
    let (span, long) = if across { (count as f64 * ROW, WIDE) } else { ((count as f64 * STEP).max(WIDE), HEIGHT) };
    let band = span / count as f64;
    let level = |v: f64| (v.max(low) - low) / (high - low) * long;
    let at = |u: f64, v: f64| if across { Point::new(level(v), u) } else { Point::new(u, long - level(v)) };
    let mut v = low;
    while v <= high + tick / 2.0 {
        canvas.shape(Line::new(at(0.0, v), at(span, v)), None, Some(Pen::border(Tone::Border).dashed()));
        let label = Label::new(Point::ZERO, number(v), SMALL, MUTED);
        canvas.label(if across { Label { at: Point::new(level(v), span + LINE * 0.75), ..label } } else { Label { at: at(0.0, v) - (size::GAP_MD, 0.0), ..label.beside(-1.0) } });
        v += tick;
    }
    canvas.shape(Line::new(at(0.0, low), at(span, low)), None, Some(Pen::border(Tone::Border)));
    let names: Vec<String> = match &x {
        Some(a) if !a.names.is_empty() => a.names.clone(),
        Some(Axis { range: Some((lo, hi)), .. }) => (0..count).map(|i| number(lo + (hi - lo) * i as f64 / (count.max(2) - 1) as f64)).collect(),
        _ => (1..=count).map(|i| i.to_string()).collect(),
    };
    for (i, name) in names.iter().take(count).enumerate() {
        let u = band * (i as f64 + 0.5);
        let label = Label::new(Point::ZERO, name.as_str(), SMALL, MUTED);
        canvas.label(if across { Label { at: Point::new(-size::GAP_MD, u), ..label.beside(-1.0) } } else { Label { at: Point::new(u, long + LINE * 0.75), ..label } });
    }
    let bars: Vec<&Series> = series.iter().filter(|s| s.bar).collect();
    let thick = band * BARS / bars.len().max(1) as f64;
    let r = size::RADIUS_SM;
    let radii = if across { RoundedRectRadii::new(0.0, r, r, 0.0) } else { RoundedRectRadii::new(r, r, 0.0, 0.0) };
    for (k, s) in bars.iter().enumerate() {
        for (i, &value) in s.values.iter().enumerate() {
            let u0 = band * i as f64 + band * (1.0 - BARS) / 2.0 + thick * k as f64;
            canvas.shape(RoundedRect::from_rect(Rect::from_points(at(u0, low), at(u0 + thick - 2.0, value)), radii), Some(Tone::Ink(hue(k))), None);
        }
    }
    for (k, s) in series.iter().filter(|s| !s.bar).enumerate() {
        let ink = Tone::Ink(hue(bars.len() + k));
        let dots: Vec<Point> = s.values.iter().enumerate().map(|(i, &v)| at(band * (i as f64 + 0.5), v)).collect();
        let mut path = BezPath::new();
        dots.iter().enumerate().for_each(|(i, &p)| if i == 0 { path.move_to(p) } else { path.line_to(p) });
        canvas.shape(path, None, Some(Pen::new(ink, 2.0)));
        dots.iter().for_each(|&p| canvas.shape(Circle::new(p, DOT), Some(ink), None));
    }
    let (wide, tall) = if across { (long, span) } else { (span, long) };
    let (below, beside) = if across { (y, x) } else { (x, y) };
    if let Some(a) = below.filter(|a| !a.title.is_empty()) {
        canvas.label(Label::new(Point::new(wide / 2.0, tall + LINE * 2.0), a.title, FONT - 1.0, MUTED));
    }
    if let Some(a) = beside.filter(|a| !a.title.is_empty()) {
        canvas.label(Label::new(Point::new(0.0, -LINE * 0.75), a.title, FONT - 1.0, MUTED).start(0.0));
    }
    if let Some(title) = title {
        canvas.label(Label::new(Point::new(wide / 2.0, -LINE * 2.0), title, FONT + 1.0, TEXT).bold());
    }
}
