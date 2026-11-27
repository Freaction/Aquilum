mod plan;
#[cfg(test)]
mod tests;

use masonry::kurbo::{Circle, Line, Point, Rect, RoundedRect, RoundedRectRadii};
use mermaid_rs_renderer::Graph;

use super::super::super::markdown::{ON_ACCENT, TEXT};
use super::super::panel::Tone as Hue;
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINK, Label, Pen};
use super::{arrow, graph};
use crate::i18n::t;
use crate::ui::tokens::size;
use plan::{INSET, LOOP, Plan, TAB, lead, plan, tab, words};

const NUMBER: f64 = 9.0;

fn frames(graph: &Graph, plan: &Plan, backgrounds: bool, canvas: &mut Canvas) {
    let accent = Pen::border(Tone::Ink(Hue::Accent));
    for (k, r, dividers) in &plan.frames {
        let frame = &graph.sequence_frames[*k];
        match (words(frame.kind), backgrounds) {
            (None, true) => canvas.shape(RoundedRect::from_rect(*r, size::RADIUS_XL), Some(Tone::Panel(Hue::Accent)), None),
            (Some((first, next)), false) => {
                canvas.shape(RoundedRect::from_rect(*r, size::RADIUS_XL), None, Some(accent));
                let heads = std::iter::once((r.y0, first, size::RADIUS_XL)).chain(dividers.iter().map(|y| (*y, next, 0.0)));
                for ((y, word, corner), section) in heads.zip(&frame.sections) {
                    if corner == 0.0 {
                        canvas.shape(Line::new((r.x0, y), (r.x1, y)), None, Some(accent.dashed()));
                    }
                    let (word, condition) = (t(word), section.label.clone().unwrap_or_default());
                    let plate = Rect::new(r.x0, y, r.x0 + tab(&word, &condition), y + TAB);
                    canvas.shape(RoundedRect::from_rect(plate, RoundedRectRadii::new(corner, 0.0, size::RADIUS_LG, 0.0)), Some(Tone::Accent), None);
                    let mid = plate.center().y;
                    canvas.label(Label::new(Point::new(plate.x0 + lead(&word), mid), condition, FONT - 1.0, ON_ACCENT).start(0.0));
                    canvas.label(Label::new(Point::new(plate.x0 + INSET * 0.75, mid), word, FONT - 1.0, ON_ACCENT).bold().start(0.0));
                }
            }
            _ => {}
        }
    }
}

pub(super) fn draw(graph: &Graph, canvas: &mut Canvas) {
    let plan = plan(graph);
    let Some(first) = plan.columns.first() else { return };
    let x = |id: &String| graph.sequence_participants.iter().position(|p| p == id).map_or(first.x, |k| plan.columns[k].x);
    let border = Pen::border(Tone::Border);
    frames(graph, &plan, true, canvas);
    for c in &plan.columns {
        canvas.shape(Line::new((c.x, plan.head), (c.x, plan.bottom)), None, Some(Pen::new(Tone::Guide, LINK)));
    }
    frames(graph, &plan, false, canvas);
    for (i, (e, (y, label))) in graph.edges.iter().zip(&plan.messages).enumerate() {
        let (a, b) = (x(&e.from), x(&e.to));
        let numbered = graph.sequence_autonumber.map(|first| first + i);
        let lead = if numbered.is_some() { NUMBER + size::GAP_XS } else { 0.0 };
        let (points, text) = if e.from == e.to {
            let points = vec![Point::new(a + lead, *y), Point::new(a + LOOP, *y), Point::new(a + LOOP, y + LOOP * 0.8), Point::new(a, y + LOOP * 0.8)];
            (points, Label::block(Point::new(a + LOOP + size::GAP_LG, y + LOOP * 0.4), label, FONT, TEXT).start(label.width))
        } else {
            let (dir, above) = ((b - a).signum(), label.height() / 2.0 + size::GAP_XS);
            (vec![Point::new(a + lead * dir, *y), Point::new(b, *y)], Label::block(Point::new((a + b) / 2.0, y - above), label, FONT, TEXT))
        };
        arrow::message(e.style, points, canvas);
        canvas.label(text);
        if let Some(number) = numbered {
            canvas.shape(Circle::new((a, *y), NUMBER), Some(Tone::Accent), None);
            canvas.label(Label::new(Point::new(a, *y), number.to_string(), FONT - 3.0, ON_ACCENT).bold());
        }
    }
    for (r, text) in &plan.notes {
        graph::note(canvas, *r, text);
    }
    for c in &plan.columns {
        let r = Rect::new(c.x - c.width / 2.0, 0.0, c.x + c.width / 2.0, plan.head);
        canvas.shape(RoundedRect::from_rect(r, size::RADIUS_XL), Some(Tone::Editor), Some(border));
        canvas.label(Label::block(r.center(), &c.name, FONT, TEXT).bold());
    }
}
