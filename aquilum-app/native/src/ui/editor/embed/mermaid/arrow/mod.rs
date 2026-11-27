mod curve;
mod head;
mod loops;
mod marks;
#[cfg(test)]
mod tests;

use std::collections::HashMap;

use masonry::kurbo::{ParamCurve, Point, Rect};
use mermaid_rs_renderer::EdgeStyle;

use super::super::tone::Tone;
use super::canvas::{Canvas, chip_rect};
use super::scene::Wire;
use super::shape::{Node, Port, Side};
pub(super) use curve::{Arrow, bow, find};
use head::End;
pub(super) use loops::{looped, room};
pub(super) use marks::{marked, marks};

const SAMPLES: usize = 20;
const CLEAR: f64 = 4.0;

pub(super) fn message(style: EdgeStyle, points: Vec<Point>, canvas: &mut Canvas) {
    let ends = [End::None, End::Arrow];
    if let Some(arrow) = Arrow::along(points, ends.map(End::inset)) {
        arrow.draw(head::pen(style, Tone::Edge), ends, canvas);
    }
}

fn ends(wire: &Wire, nodes: &HashMap<&str, Node>, vertical: bool) -> Option<[Port; 2]> {
    let (edge, p) = (&wire.edge, &wire.points);
    let n = p.len();
    let linked = edge.from != edge.to && n > 1 && nodes.contains_key(edge.from.as_str()) && nodes.contains_key(edge.to.as_str());
    let [a, b] = wire.sides.unwrap_or_else(|| [Side::along(p[1] - p[0], vertical), Side::along(p[n - 2] - p[n - 1], vertical)]);
    linked.then(|| [Port { point: p[0], side: a }, Port { point: p[n - 1], side: b }])
}

pub(super) fn sides(from: Node, to: Node, walls: &[Rect]) -> [Side; 2] {
    let (a, b) = (from.rect, to.rect);
    let (ca, cb) = (a.center(), b.center());
    let (right, down) = (cb.x > ca.x, cb.y > ca.y);
    let across = if right { [Side::Right, Side::Left] } else { [Side::Left, Side::Right] };
    let upright = if down { [Side::Bottom, Side::Top] } else { [Side::Top, Side::Bottom] };
    let room_x = |from: f64, to: Rect| if right { to.x0 - from } else { from - to.x1 };
    let room_y = |from: f64, to: Rect| if down { to.y0 - from } else { from - to.y1 };
    let options = [
        ((b.x0 - a.x1).max(a.x0 - b.x1), across),
        ((b.y0 - a.y1).max(a.y0 - b.y1), upright),
        (room_x(ca.x, b).min(if down { cb.y - a.y1 } else { a.y0 - cb.y }), [upright[0], across[1]]),
        (room_y(ca.y, b).min(if right { cb.x - a.x1 } else { a.x0 - cb.x }), [across[0], upright[1]]),
    ];
    let clear = |[sa, sb]: [Side; 2]| {
        let curve = bow(from.port(sa, 0.5), sa.normal(), to.port(sb, 0.5), sb.normal());
        (1..SAMPLES).map(|k| curve.eval(k as f64 / SAMPLES as f64)).all(|p| walls.iter().all(|w| !w.inflate(CLEAR, CLEAR).contains(p)))
    };
    let score = |(room, pair): &(f64, [Side; 2])| (clear(*pair), *room);
    options.into_iter().max_by(|x, y| score(x).partial_cmp(&score(y)).unwrap_or(std::cmp::Ordering::Equal)).map_or(upright, |o| o.1)
}

pub(super) fn chip(wire: &Wire, arrow: &Arrow) -> Option<Rect> {
    wire.label.as_ref().map(|text| chip_rect(wire.anchor.unwrap_or_else(|| arrow.at(0.5)), text))
}

pub(super) fn lines(wires: &[Wire], nodes: &HashMap<&str, Node>, vertical: bool) -> Vec<Option<Arrow>> {
    wires
        .iter()
        .map(|wire| {
            let insets = End::pair(&wire.edge).map(End::inset);
            match ends(wire, nodes, vertical) {
                Some(ends) => Some(Arrow::curve(ends, insets)),
                None if wire.edge.from == wire.edge.to => nodes.get(wire.edge.from.as_str()).and_then(|n| loops::arrow(&wire.points, n.rect.center(), insets)),
                None => Arrow::along(wire.points.clone(), insets),
            }
        })
        .collect()
}

pub(super) fn edges(wires: &[Wire], nodes: &HashMap<&str, Node>, vertical: bool, tone: impl Fn(&Wire) -> Tone, canvas: &mut Canvas) {
    for (wire, arrow) in wires.iter().zip(lines(wires, nodes, vertical)) {
        let Some(arrow) = arrow else { continue };
        let ends = End::pair(&wire.edge);
        arrow.draw(head::pen(wire.edge.style, tone(wire)), ends, canvas);
        if let (Some(text), Some(spot)) = (&wire.label, chip(wire, &arrow)) {
            canvas.chip(spot.center(), text);
        }
        for label in marks(wire, &arrow, nodes) {
            canvas.label(label);
        }
    }
}
