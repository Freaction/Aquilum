use masonry::kurbo::{Circle, Line, Point, Rect, RoundedRect};
use mermaid_rs_renderer::EdgeStyle;

use super::super::super::markdown::TEXT;
use super::super::tone::Tone;
use super::canvas::{BOLD, Canvas, FONT, LINE, LINK, Label, Pen, Text, wrap};
use super::columns::{GAP, columns};
use super::{arrow, hue, titled};
use crate::ui::tokens::size;

const WRAP: f64 = 160.0;
const COLUMN: f64 = 112.0;
const STEM: f64 = 20.0;
const DOT: f64 = 5.0;

struct Period {
    name: Text,
    events: Vec<Text>,
    section: Option<usize>,
}

fn text(s: &str) -> Text {
    wrap(s, WRAP)
}

fn tall(block: &Text) -> f64 {
    block.height().max(LINE) + size::GAP_MD * 2.0
}

fn parse<'a>(lines: &[&'a str]) -> (Option<&'a str>, Vec<&'a str>, Vec<Period>) {
    let (title, body) = titled(lines);
    let (mut sections, mut periods) = (Vec::new(), Vec::<Period>::new());
    for line in body {
        if let Some(rest) = line.strip_prefix("section ") {
            sections.push(rest.trim());
        } else if !line.starts_with("acc") {
            let mut parts = line.split(':').map(str::trim);
            let first = parts.next().unwrap_or_default();
            let events = parts.filter(|e| !e.is_empty()).map(text);
            match periods.last_mut() {
                Some(last) if first.is_empty() => last.events.extend(events),
                _ if first.is_empty() => {}
                _ => periods.push(Period { name: text(first), events: events.collect(), section: sections.len().checked_sub(1) }),
            }
        }
    }
    (title, sections, periods)
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    let (title, sections, periods) = parse(lines);
    let Some(last) = periods.len().checked_sub(1) else { return };
    let tint = |k: usize| hue(periods[k].section.unwrap_or(k));
    let widths = periods.iter().map(|p| p.events.iter().map(|e| e.width).fold(p.name.width * BOLD, f64::max).max(COLUMN) + size::GAP_XL * 2.0).collect();
    let of: Vec<_> = periods.iter().map(|p| p.section).collect();
    let (lefts, widths, y) = columns(title, &sections, &of, widths, canvas);
    let head = periods.iter().map(|p| tall(&p.name)).fold(0.0, f64::max);
    let axis = y + head + STEM;
    arrow::message(EdgeStyle::Solid, vec![Point::new(0.0, axis), Point::new(lefts[last] + widths[last] + GAP, axis)], canvas);
    for (k, p) in periods.iter().enumerate() {
        let (x0, w) = (lefts[k], widths[k]);
        let mid = x0 + w / 2.0;
        let low = p.events.iter().map(tall).sum::<f64>() + size::GAP_MD * p.events.len().saturating_sub(1) as f64;
        let stem = if p.events.is_empty() { axis } else { axis + STEM + low - tall(&p.events[p.events.len() - 1]) };
        canvas.shape(Line::new((mid, y + head), (mid, stem)), None, Some(Pen::new(Tone::Guide, LINK)));
        let chip = Rect::new(x0, y, x0 + w, y + head);
        canvas.shape(RoundedRect::from_rect(chip, size::RADIUS_LG), Some(Tone::Panel(tint(k))), None);
        canvas.label(Label::block(chip.center(), &p.name, FONT, TEXT).bold());
        canvas.shape(Circle::new((mid, axis), DOT), Some(Tone::Ink(tint(k))), None);
        let mut top = axis + STEM;
        for e in &p.events {
            let card = Rect::new(x0, top, x0 + w, top + tall(e));
            canvas.shape(RoundedRect::from_rect(card, size::RADIUS_LG), Some(Tone::Editor), Some(Pen::border(Tone::Border)));
            canvas.label(Label::block(card.center(), e, FONT, TEXT));
            top = card.y1 + size::GAP_MD;
        }
    }
}
