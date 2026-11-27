use std::collections::{HashMap, HashSet};

use masonry::kurbo::Point;
use mermaid_rs_renderer::{Direction, Graph, NodeShape};

use super::super::arrow;
use super::super::shape::{Node, Port, Side};
use super::net::{Kind, Net};
use super::{at, back, frame, rect};

const RAIL: f64 = 24.0;
const MARKED: f64 = 56.0;

#[derive(Clone, Copy)]
pub(super) struct Tip<'a> {
    pub id: &'a str,
    pub node: Node,
    pub side: Side,
    pub toward: Point,
    pub marked: bool,
}

pub(super) fn plan(links: &[Option<[Tip<'_>; 2]>]) -> Vec<Option<[Port; 2]>> {
    let marked: HashSet<(&str, Side)> = links.iter().flatten().flatten().filter(|t| t.marked).map(|t| (t.id, t.side)).collect();
    let mut slots: HashMap<(&str, Side, Option<&str>), Vec<(f64, usize, usize)>> = HashMap::new();
    for (i, tips) in links.iter().enumerate() {
        let Some(pair) = tips else { continue };
        for (end, tip) in pair.iter().enumerate() {
            let along = if matches!(tip.side, Side::Top | Side::Bottom) { tip.toward.x } else { tip.toward.y };
            let other = (!marked.contains(&(tip.id, tip.side))).then_some(pair[1 - end].id);
            slots.entry((tip.id, tip.side, other)).or_default().push((along, i, end));
        }
    }
    let mut ports = vec![[None; 2]; links.len()];
    for ((_, side, other), mut list) in slots {
        let rail = if other.is_some() { RAIL } else { MARKED };
        list.sort_by(|x, y| x.0.total_cmp(&y.0));
        let middle = (list.len() as f64 - 1.0) / 2.0;
        for (k, &(_, i, end)) in list.iter().enumerate() {
            let Some(tips) = links[i] else { continue };
            let r = tips[end].node.rect;
            let length = if matches!(side, Side::Top | Side::Bottom) { r.width() } else { r.height() };
            let t = (0.5 + (k as f64 - middle) * rail / length.max(1.0)).clamp(0.1, 0.9);
            ports[i][end] = Some(Port { point: tips[end].node.port(side, t), side });
        }
    }
    ports.into_iter().map(|[a, b]| a.zip(b).map(|(a, b)| [a, b])).collect()
}

fn end<'a>(net: &'a Net, graph: &'a Graph, direction: Direction, item: usize) -> (&'a str, Node) {
    let i = &net.items[item];
    let (id, rect) = match i.kind {
        Kind::Top | Kind::Bottom => (graph.subgraphs[net.groups[i.key].index].id.as_deref().unwrap_or_default(), frame(direction, net.frame(i.key))),
        _ => (net.nodes[i.key].0.as_str(), rect(net, direction, item)),
    };
    (id, Node { rect, shape: graph.nodes.get(id).map_or(NodeShape::Rectangle, |n| n.shape) })
}

pub(super) fn ports(net: &Net, graph: &Graph, direction: Direction) -> Vec<[Point; 2]> {
    let side = |y: f64| Side::along(at(direction, 0.0, y).to_vec2(), !net.across);
    let toward = |i: usize| at(direction, net.items[i].x, net.items[i].y);
    let links: Vec<_> = net
        .wires
        .iter()
        .map(|w| {
            let (p, n) = (&w.path, w.path.len());
            let ((ia, a), (ib, b)) = (end(net, graph, direction, p[0]), end(net, graph, direction, p[n - 1]));
            let [first, last] = arrow::marked(&graph.edges[w.edge]);
            let (ma, mb) = if p[0] == w.from { (first, last) } else { (last, first) };
            Some([Tip { id: ia, node: a, side: side(1.0), toward: toward(p[1]), marked: ma }, Tip { id: ib, node: b, side: side(-1.0), toward: toward(p[n - 2]), marked: mb }])
        })
        .collect();
    plan(&links).into_iter().map(|p| p.map_or([Point::ZERO; 2], |[a, b]| [back(direction, a.point), back(direction, b.point)])).collect()
}
