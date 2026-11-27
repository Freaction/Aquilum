use std::f64::consts::{FRAC_PI_2, TAU};

use masonry::kurbo::{Arc, BezPath, Circle, Point, Rect, RoundedRect, Vec2};
use mermaid_rs_renderer::Graph;

use super::super::super::markdown::TEXT;
use super::super::tone::Tone;
use super::{PALETTE, number};
use super::canvas::{Canvas, FONT, Label, Pen, width};
use crate::i18n::t;
use crate::ui::tokens::size;

const HOLE: f64 = 0.58;
const SEAM: f64 = 2.0;
const TOLERANCE: f64 = 0.1;
const MARKER: f64 = 12.0;
const RADIUS: f64 = 152.0;
const TITLE: f64 = 37.5;

fn sector(center: Point, outer: f64, start: f64, end: f64) -> BezPath {
    let inner = outer * HOLE;
    let mut path = BezPath::new();
    path.move_to(center + Vec2::from_angle(start) * outer);
    path.extend(Arc::new(center, (outer, outer), start, end - start, 0.0).append_iter(TOLERANCE));
    path.line_to(center + Vec2::from_angle(end) * inner);
    path.extend(Arc::new(center, (inner, inner), end, start - end, 0.0).append_iter(TOLERANCE));
    path.close_path();
    path
}

fn shares(graph: &Graph) -> Vec<(String, f64)> {
    let mut items: Vec<(String, f64)> = graph.pie_slices.iter().map(|s| (s.label.clone(), f64::from(s.value))).filter(|(_, v)| *v > 0.0).collect();
    if items.len() > PALETTE.len() {
        let rest = items.split_off(PALETTE.len() - 1).into_iter().map(|(_, v)| v).sum();
        items.push((t("diagram.other"), rest));
    }
    items
}

pub(super) fn draw(graph: &Graph, room: f64, canvas: &mut Canvas) {
    let (center, radius) = (Point::ORIGIN, RADIUS);
    let items = shares(graph);
    let total: f64 = items.iter().map(|(_, v)| v).sum();
    let seam = Pen::new(Tone::Editor, SEAM);
    let mut angle = -FRAC_PI_2;
    for (i, (_, value)) in items.iter().enumerate() {
        let sweep = value / total * TAU;
        if items.len() == 1 {
            canvas.shape(Circle::new(center, radius * (1.0 + HOLE) / 2.0), None, Some(Pen::new(Tone::Ink(PALETTE[0]), radius * (1.0 - HOLE))));
        } else {
            canvas.shape(sector(center, radius, angle, angle + sweep), Some(Tone::Ink(PALETTE[i])), Some(seam));
        }
        angle += sweep;
    }
    if graph.pie_show_data && total > 0.0 {
        canvas.label(Label::new(center, number(total), FONT + 6.0, TEXT).bold());
    }
    if let Some(title) = &graph.pie_title {
        let y = center.y - radius - size::GAP_2XL - TITLE / 2.0;
        canvas.label(Label::new(Point::new(center.x, y), title.clone(), FONT + 1.0, TEXT).bold());
    }
    let texts: Vec<String> = items
        .iter()
        .map(|(label, value)| {
            let data = if graph.pie_show_data { format!(" [{}]", number(*value)) } else { String::new() };
            format!("{label}{data} · {:.0}%", value / total * 100.0)
        })
        .collect();
    let widest = texts.iter().map(|text| width(text, FONT)).fold(0.0, f64::max) + MARKER + size::GAP_LG;
    let step = FONT * 1.7;
    let stacked = radius * 2.0 + size::GAP_3XL + widest > room;
    let (x, top) = if stacked {
        (center.x - radius, center.y + radius + size::GAP_3XL)
    } else {
        (center.x + radius + size::GAP_3XL, center.y - (texts.len() as f64 - 1.0) * step / 2.0)
    };
    for (i, text) in texts.into_iter().enumerate() {
        let y = top + i as f64 * step;
        let marker = Rect::new(x, y - MARKER / 2.0, x + MARKER, y + MARKER / 2.0);
        canvas.shape(RoundedRect::from_rect(marker, size::RADIUS_MD), Some(Tone::Ink(PALETTE[i])), None);
        canvas.label(Label::new(Point::new(marker.x1 + size::GAP_LG, y), text, FONT, TEXT).start(0.0));
    }
}
