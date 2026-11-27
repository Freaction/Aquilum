use masonry::kurbo::{BezPath, Circle, Point, Rect, RoundedRect, Vec2};

use super::super::super::markdown::{MUTED, ON_ACCENT, TEXT};
use super::super::panel::Tone as Hue;
use super::super::tone::Tone;
use super::arrow::bow;
use super::canvas::{Canvas, FONT, LINK, Label, Pen, Text, wrap};
use super::columns::{GAP, columns};
use super::{titled, unquote};
use crate::ui::tokens::size;

const WRAP: f64 = 140.0;
const COLUMN: f64 = 96.0;
const LEVEL: f64 = 22.0;
const FACE: f64 = 10.0;
const SMALL: f64 = FONT - 2.0;

struct Step {
    name: Text,
    score: u8,
    actors: String,
    section: Option<usize>,
}

fn mood(score: u8) -> Hue {
    match score {
        4.. => Hue::Success,
        3 => Hue::Warning,
        _ => Hue::Danger,
    }
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (title, body) = titled(lines);
    let (mut sections, mut steps) = (Vec::new(), Vec::<Step>::new());
    for line in body {
        if let Some(rest) = line.strip_prefix("section ") {
            sections.push(unquote(rest));
        } else if let Some((name, rest)) = line.split_once(':') {
            let (score, actors) = rest.split_once(':').unwrap_or((rest, ""));
            let score = score.trim().parse::<u8>().unwrap_or(3).clamp(1, 5);
            let actors = actors.split(',').map(str::trim).filter(|a| !a.is_empty()).collect::<Vec<_>>().join(", ");
            steps.push(Step { name: wrap(unquote(name), WRAP), score, actors, section: sections.len().checked_sub(1) });
        }
    }
    if steps.is_empty() {
        return;
    }
    let widths = steps.iter().map(|s| s.name.width.max(COLUMN) + size::GAP_XL * 2.0).collect();
    let of: Vec<_> = steps.iter().map(|s| s.section).collect();
    let (lefts, widths, y) = columns(title, &sections, &of, widths, canvas);
    let card = steps.iter().map(|s| s.name.height()).fold(0.0, f64::max) + size::GAP_MD * 2.0;
    let top = y + card + GAP + FACE;
    let faces: Vec<Point> = steps.iter().enumerate().map(|(k, s)| Point::new(lefts[k] + widths[k] / 2.0, top + f64::from(5 - s.score) * LEVEL)).collect();
    for pair in faces.windows(2) {
        let curve = bow(pair[0], Vec2::new(1.0, 0.0), pair[1], Vec2::new(-1.0, 0.0));
        let mut path = BezPath::new();
        path.move_to(pair[0]);
        path.curve_to(curve.p1, curve.p2, pair[1]);
        canvas.shape(path, None, Some(Pen::new(Tone::Edge, LINK)));
    }
    let actors = top + LEVEL * 4.0 + FACE + GAP;
    for (k, s) in steps.iter().enumerate() {
        let r = Rect::new(lefts[k], y, lefts[k] + widths[k], y + card);
        canvas.shape(RoundedRect::from_rect(r, size::RADIUS_LG), Some(Tone::Editor), Some(Pen::border(Tone::Border)));
        canvas.label(Label::block(r.center(), &s.name, FONT, TEXT));
        canvas.shape(Circle::new(faces[k], FACE), Some(Tone::Ink(mood(s.score))), None);
        canvas.label(Label::new(faces[k], s.score.to_string(), SMALL, ON_ACCENT).bold());
        if !s.actors.is_empty() {
            canvas.label(Label { width: widths[k], ..Label::new(Point::new(r.center().x, actors), s.actors.as_str(), SMALL, MUTED) });
        }
    }
}
