use std::collections::HashMap;

use masonry::core::BrushIndex;
use masonry::kurbo::{Circle, Line, Point, Rect, RoundedRect, RoundedRectRadii};
use mermaid_rs_renderer::{DiagramKind, Direction, Edge, EdgeStyle, Graph, NodeShape};

use super::super::super::markdown::{ON_ACCENT, SECONDARY, TEXT};
use super::super::panel::Tone as Hue;
use super::super::tone::Tone;
use super::canvas::{Canvas, FONT, LINK, Label, Pen, clean};
use super::scene::{Placed, Scene, Wire};
use super::shape::{Node, Side};
use super::{arrow, hue, layout};
use crate::ui::tokens::size;

const DOT: f64 = 7.0;

pub(super) fn note(canvas: &mut Canvas, r: Rect, text: &str) {
    canvas.shape(RoundedRect::from_rect(r, size::RADIUS_LG), Some(Tone::Panel(Hue::Warning)), None);
    canvas.label(Label { width: r.width(), ..Label::new(r.center(), text, FONT, TEXT) });
}

fn look(shape: NodeShape) -> (Option<Tone>, Option<Tone>) {
    let hue = |hue: Hue| (Some(Tone::Panel(hue)), Some(Tone::Ink(hue)));
    match shape {
        NodeShape::Text => (None, None),
        NodeShape::Stadium | NodeShape::Circle | NodeShape::DoubleCircle => hue(Hue::Accent),
        NodeShape::Diamond | NodeShape::Hexagon => hue(Hue::Warning),
        NodeShape::Cylinder => hue(Hue::Info),
        NodeShape::ForkJoin => (Some(Tone::Edge), None),
        _ => (Some(Tone::Editor), Some(Tone::Border)),
    }
}

pub(super) fn draw(graph: &Graph, scene: &Scene, loops: &[Edge], canvas: &mut Canvas) {
    let mindmap = (graph.kind == DiagramKind::Mindmap).then(|| graph.mindmap.nodes.iter().map(|n| (n.id.as_str(), (n.level, hue(n.section.unwrap_or(0))))).collect::<HashMap<_, _>>());
    let tint = |id: &str| mindmap.as_ref().and_then(|m| m.get(id).copied());
    for frame in &scene.frames {
        let r = frame.rect;
        canvas.shape(RoundedRect::from_rect(r, size::RADIUS_2XL), None, Some(Pen::border(Tone::Border)));
        let at = Point::new(r.x0 + size::GAP_XL, r.y0 + frame.title.height() / 2.0 + size::GAP_MD);
        canvas.label(Label::block(at, &frame.title, FONT - 1.0, SECONDARY).start(frame.title.width).bold());
    }
    let (nodes, wires, vertical) = prepare(graph, scene, loops);
    arrow::edges(&wires, &nodes, vertical, |w| tint(&w.edge.to).map_or(Tone::Edge, |(_, h)| Tone::Ink(h)), canvas);
    for (id, n) in scene.nodes.iter().filter(|(_, n)| !n.hidden) {
        let center = n.rect.center();
        match tint(id) {
            Some((0, _)) => plain(n, (Some(Tone::Accent), None), ON_ACCENT, canvas),
            Some((_, h)) => plain(n, (Some(Tone::Panel(h)), Some(Tone::Ink(h))), TEXT, canvas),
            None if id.starts_with("__start_") => canvas.shape(Circle::new(center, DOT), Some(Tone::Accent), None),
            None if id.starts_with("__end_") => {
                canvas.shape(Circle::new(center, DOT + size::GAP_SM), None, Some(Pen::new(Tone::Accent, LINK)));
                canvas.shape(Circle::new(center, DOT - size::GAP_XS), Some(Tone::Accent), None);
            }
            None if n.label.lines.iter().any(|l| l.trim() == "---") => compartments(n, canvas),
            None => plain(n, look(n.shape), TEXT, canvas),
        }
    }
    for (r, text) in &scene.notes {
        note(canvas, *r, text);
    }
}

pub(super) fn edge(from: &str, to: &str, label: Option<String>, (start, end): (bool, bool)) -> Edge {
    let (from, to) = (from.to_owned(), to.to_owned());
    let (start_label, end_label, arrow_start_kind, arrow_end_kind, start_decoration, end_decoration) = (None, None, None, None, None, None);
    Edge { from, to, label, start_label, end_label, directed: end, arrow_start: start, arrow_end: end, arrow_start_kind, arrow_end_kind, start_decoration, end_decoration, style: EdgeStyle::Solid }
}

pub(super) fn render(graph: &Graph, canvas: &mut Canvas) {
    if graph.nodes.is_empty() {
        return;
    }
    let scene = layout::lay(graph, &[], 1.0);
    draw(graph, &scene, &[], canvas);
}

pub(super) fn prepare<'a>(graph: &Graph, scene: &'a Scene, loops: &[Edge]) -> (HashMap<&'a str, Node>, Vec<Wire>, bool) {
    let nodes: HashMap<&str, Node> = scene.nodes.iter().map(|(id, n)| (id.as_str(), Node { rect: n.rect, shape: n.shape })).collect();
    let vertical = matches!(graph.direction, Direction::TopDown | Direction::BottomTop);
    let side = if vertical { Side::Right } else { Side::Bottom };
    let looped = loops.iter().filter_map(|e| nodes.get(e.from.as_str()).map(|n| arrow::looped(e, n.rect, side)));
    let wires = scene.wires.iter().cloned().chain(looped).collect();
    (nodes, wires, vertical)
}

fn plain(n: &Placed, (fill, stroke): (Option<Tone>, Option<Tone>), brush: BrushIndex, canvas: &mut Canvas) {
    let node = Node { rect: n.rect, shape: n.shape };
    let pen = stroke.map(Pen::border);
    if let Some(outline) = node.outline() {
        canvas.shape(outline, fill, pen);
    }
    if let (Some(details), Some(pen)) = (node.details(), pen) {
        canvas.shape(details, None, Some(pen));
    }
    let bold = brush == ON_ACCENT;
    let label = Label::block(n.rect.center(), &n.label, FONT, brush);
    canvas.label(if bold { label.bold() } else { label });
}

fn compartments(n: &Placed, canvas: &mut Canvas) {
    let (lines, r) = (&n.label.lines, n.rect);
    let row = |i: usize| r.y0 + (i as f64 + 1.0) * r.height() / (lines.len() as f64 + 1.0);
    let header = lines.iter().position(|l| l.trim() == "---").unwrap_or(lines.len());
    let radius = size::RADIUS_LG;
    canvas.shape(RoundedRect::from_rect(r, radius), Some(Tone::Editor), None);
    canvas.shape(RoundedRect::from_rect(Rect::new(r.x0, r.y0, r.x1, row(header)), RoundedRectRadii::new(radius, radius, 0.0, 0.0)), Some(Tone::Panel(Hue::Accent)), None);
    canvas.shape(RoundedRect::from_rect(r, radius), None, Some(Pen::border(Tone::Ink(Hue::Accent))));
    let inner = r.width() - size::GAP_XL * 2.0;
    for (i, line) in lines.iter().enumerate() {
        let y = row(i);
        if line.trim() == "---" {
            if i != header {
                canvas.shape(Line::new((r.x0, y), (r.x1, y)), None, Some(Pen::border(Tone::Border)));
            }
        } else if i < header {
            canvas.label(Label { width: inner, ..Label::new(Point::new(r.center().x, y), clean(line), FONT, TEXT).bold() });
        } else {
            canvas.label(Label::new(Point::new(r.x0 + size::GAP_XL, y), clean(line), FONT - 1.0, TEXT).start(inner));
        }
    }
}
