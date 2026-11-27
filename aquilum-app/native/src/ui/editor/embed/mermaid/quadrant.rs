use masonry::kurbo::{Circle, Point, Rect, RoundedRect, RoundedRectRadii};

use super::super::super::markdown::{MUTED, TEXT};
use super::super::panel::Tone as Hue;
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINE, Label};
use super::{titled, unquote};
use crate::ui::tokens::size;

const SIZE: f64 = 360.0;
const DOT: f64 = 5.0;
const SEAM: f64 = 3.0;
const HUES: [Hue; 4] = [Hue::Success, Hue::Accent, Hue::Warning, Hue::Info];

fn axis(rest: &str) -> [String; 2] {
    let (low, high) = rest.split_once("-->").unwrap_or((rest, ""));
    [unquote(low).to_owned(), unquote(high).to_owned()]
}

fn point(rest: &str) -> Option<(f64, f64)> {
    let inside = &rest[rest.find('[')? + 1..rest.find(']')?];
    let (x, y) = inside.split_once(',')?;
    Some((x.trim().parse::<f64>().ok()?.clamp(0.0, 1.0), y.trim().parse::<f64>().ok()?.clamp(0.0, 1.0)))
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (title, body) = titled(lines);
    let (mut xs, mut ys, mut names, mut points) = ([String::new(), String::new()], [String::new(), String::new()], [""; 4], Vec::new());
    for line in body {
        if let Some(rest) = line.strip_prefix("x-axis") {
            xs = axis(rest);
        } else if let Some(rest) = line.strip_prefix("y-axis") {
            ys = axis(rest);
        } else if let Some((n, text)) = line.strip_prefix("quadrant-").and_then(|r| r.split_once(' ')) {
            if let Some(slot) = n.parse::<usize>().ok().and_then(|n| n.checked_sub(1)).and_then(|n| names.get_mut(n)) {
                *slot = unquote(text);
            }
        } else if let Some((name, rest)) = line.split_once(':') {
            points.extend(point(rest).map(|(x, y)| (unquote(name), x, y)));
        }
    }
    if points.is_empty() && names.iter().all(|n| n.is_empty()) {
        return;
    }
    let half = SIZE / 2.0;
    let r = size::RADIUS_XL;
    let cells = [(half, 0.0, [0.0, r, 0.0, 0.0]), (0.0, 0.0, [r, 0.0, 0.0, 0.0]), (0.0, half, [0.0, 0.0, 0.0, r]), (half, half, [0.0, 0.0, r, 0.0])];
    for (k, &(x, y, [a, b, c, d])) in cells.iter().enumerate() {
        let cell = Rect::new(x, y, x + half, y + half).inflate(-SEAM, -SEAM);
        canvas.shape(RoundedRect::from_rect(cell, RoundedRectRadii::new(a, b, c, d)), Some(Tone::Panel(HUES[k])), None);
        canvas.label(Label { width: half - size::GAP_XL * 2.0, ..Label::new(Point::new(cell.center().x, y + size::GAP_XL + LINE / 2.0), names[k], FONT, TEXT).bold() });
    }
    for (name, x, y) in &points {
        let at = Point::new(x * SIZE, (1.0 - y) * SIZE);
        canvas.shape(Circle::new(at, DOT), Some(Tone::Accent), None);
        canvas.label(Label::new(Point::new(at.x + DOT + size::GAP_SM, at.y), *name, FONT - 1.0, TEXT).start(0.0));
    }
    for (k, text) in xs.iter().enumerate() {
        canvas.label(Label::new(Point::new(half * (k as f64 + 0.5), SIZE + LINE), text.as_str(), FONT - 1.0, MUTED));
    }
    for (k, text) in ys.iter().enumerate() {
        canvas.label(Label::new(Point::new(-size::GAP_LG, SIZE - half * (k as f64 + 0.5)), text.as_str(), FONT - 1.0, MUTED).beside(-1.0));
    }
    if let Some(title) = title {
        canvas.label(Label::new(Point::new(half, -LINE), title, FONT + 1.0, TEXT).bold());
    }
}
