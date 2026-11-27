mod bend;
mod cycles;
mod net;
mod order;
mod place;
mod ports;
mod rank;
mod settle;
mod size;
mod solve;
mod tree;
#[cfg(test)]
mod tests;

use masonry::kurbo::{Point, Rect};
use mermaid_rs_renderer::{Direction, Edge, Graph, NodeShape};

use super::canvas::Text;
use super::scene::{Frame, Placed, Scene, Wire};
use net::{Kind, Net};
use place::Gaps;
pub(super) use tree::tree;

pub(super) const NODE_GAP: f64 = 56.0;
pub(super) const RANK_GAP: f64 = 72.0;

fn at(direction: Direction, x: f64, y: f64) -> Point {
    match direction {
        Direction::TopDown => Point::new(x, y),
        Direction::BottomTop => Point::new(x, -y),
        Direction::LeftRight => Point::new(y, x),
        Direction::RightLeft => Point::new(-y, x),
    }
}

fn back(direction: Direction, p: Point) -> Point {
    match direction {
        Direction::TopDown => p,
        Direction::BottomTop => Point::new(p.x, -p.y),
        Direction::LeftRight => Point::new(p.y, p.x),
        Direction::RightLeft => Point::new(p.y, -p.x),
    }
}

fn frame(direction: Direction, (x0, y0, x1, y1): (f64, f64, f64, f64)) -> Rect {
    Rect::from_points(at(direction, x0, y0), at(direction, x1, y1))
}

fn rect(net: &Net, direction: Direction, item: usize) -> Rect {
    let i = &net.items[item];
    frame(direction, (i.x - i.w / 2.0, i.y - i.h / 2.0, i.x + i.w / 2.0, i.y + i.h / 2.0))
}

pub(super) fn measure(shape: NodeShape, label: &str) -> (Text, (f64, f64)) {
    let text = size::text(label);
    let dims = size::node(shape, &text, false);
    (text, dims)
}

pub(super) fn lay(graph: &Graph, loops: &[Edge], spread: f64) -> Scene {
    let gaps = Gaps { node: NODE_GAP * spread, rank: RANK_GAP * spread };
    let mut net = net::build(graph, loops);
    rank::assign(&mut net);
    rank::chain(&mut net);
    let direction = graph.direction;
    let mut layers = order::layers(&net);
    order::arrange(&net, &order::lanes(&net), &mut layers);
    settle::settle(&mut net, &mut layers, &gaps, &|net| ports::ports(net, graph, direction));
    let mut scene = Scene::default();
    for (i, item) in net.items.iter().enumerate().filter(|(_, i)| i.kind == Kind::Node) {
        let (id, label) = &net.nodes[item.key];
        scene.nodes.insert(id.clone(), Placed { rect: rect(&net, direction, i), shape: graph.nodes[id].shape, label: label.clone(), hidden: false });
    }
    for (g, group) in net.groups.iter().enumerate() {
        let r = frame(direction, net.frame(g));
        let sub = &graph.subgraphs[group.index];
        if let Some(id) = sub.id.as_ref().filter(|id| graph.nodes.contains_key(*id)) {
            scene.nodes.insert(id.clone(), Placed { rect: r, shape: graph.nodes[id].shape, label: group.title.clone(), hidden: true });
        }
        scene.frames.push(Frame { rect: r, title: group.title.clone() });
    }
    for (wire, [a, b]) in net.wires.iter().zip(ports::ports(&net, graph, direction)) {
        let inner = wire.path[1..wire.path.len() - 1].iter().map(|&i| Point::new(net.items[i].x, net.items[i].y));
        let mut points: Vec<Point> = std::iter::once(a).chain(inner).chain(std::iter::once(b)).map(|p| at(direction, p.x, p.y)).collect();
        if wire.path.first() != Some(&wire.from) {
            points.reverse();
        }
        let anchor = wire.path.iter().find(|&&i| net.items[i].kind == Kind::Label).map(|&i| at(direction, net.items[i].x, net.items[i].y));
        scene.wires.push(Wire { edge: graph.edges[wire.edge].clone(), points, label: wire.label.as_ref().map(|l| l.0.clone()), anchor, sides: None });
    }
    for &(item, _, _) in &net.notes {
        let note = &graph.state_notes[net.items[item].key];
        scene.notes.push((rect(&net, direction, item), size::text(&note.label).lines.join("\n")));
    }
    scene
}
