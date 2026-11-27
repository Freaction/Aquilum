use std::collections::{BTreeSet, HashMap, VecDeque};

use masonry::kurbo::Rect;
use mermaid_rs_renderer::{Graph, NodeShape, Subgraph};

use super::canvas::{Canvas, LINE};
use super::graph::{draw as paint, edge};
use super::layout::measure;
use super::scene::{Frame, Placed, Scene, Wire};
use super::shape::{Node, Side};
use super::unquote;

const ACROSS: f64 = 96.0;
const DOWN: f64 = 80.0;
const PAD: f64 = 16.0;

type Cell = (i32, i32);

fn icon(name: &str) -> (&'static str, NodeShape) {
    match name {
        "database" => ("🗄️ ", NodeShape::Cylinder),
        "disk" => ("💾 ", NodeShape::Cylinder),
        "cloud" => ("☁️ ", NodeShape::Rectangle),
        "internet" => ("🌐 ", NodeShape::Rectangle),
        "server" => ("🖥️ ", NodeShape::Rectangle),
        _ => ("", NodeShape::Rectangle),
    }
}

fn item(rest: &str) -> (&str, &str, &str, Option<&str>) {
    let (head, parent) = rest.split_once(" in ").map_or((rest, None), |(h, p)| (h, Some(p.trim())));
    let head = head.trim();
    let id = &head[..head.find(['(', '[']).unwrap_or(head.len())];
    let between = |a: char, b: char| head.find(a).zip(head.rfind(b)).filter(|(x, y)| y > x).map(|(x, y)| &head[x + 1..y]);
    (id, between('(', ')').unwrap_or_default(), between('[', ']').map_or(id, unquote), parent)
}

fn end(part: &str) -> (&str, Side) {
    let part = part.trim().trim_matches(['<', '>']).trim();
    let (a, b) = part.split_once(':').unwrap_or((part, ""));
    let (id, side) = if a.len() == 1 && !b.is_empty() { (b, a) } else { (a, b) };
    let side = match side.trim() {
        "L" => Side::Left,
        "T" => Side::Top,
        "B" => Side::Bottom,
        _ => Side::Right,
    };
    (id.split('{').next().unwrap_or(id).trim(), side)
}

fn step(side: Side) -> Cell {
    match side {
        Side::Left => (-1, 0),
        Side::Right => (1, 0),
        Side::Top => (0, -1),
        Side::Bottom => (0, 1),
    }
}

fn put<'a>(id: &'a str, mut at: Cell, d: Cell, cells: &mut HashMap<&'a str, Cell>, taken: &mut HashMap<Cell, &'a str>) {
    while taken.contains_key(&at) {
        at = (at.0 + d.0, at.1 + d.1);
    }
    cells.insert(id, at);
    taken.insert(at, id);
}

pub(super) fn scene(lines: &[&str]) -> Option<(Graph, Scene)> {
    let mut graph = Graph::new();
    let (mut parents, mut groups, mut links) = (HashMap::new(), HashMap::new(), Vec::new());
    for line in lines.iter().map(|l| l.trim()) {
        let (word, rest) = line.split_once(char::is_whitespace).unwrap_or((line, ""));
        if matches!(word, "group" | "service" | "junction") {
            let (id, kind, label, parent) = item(rest);
            let (mark, shape) = icon(kind);
            parents.extend(parent.map(|p| (id, p)));
            match word {
                "group" => {
                    groups.insert(id, graph.subgraphs.len());
                    graph.subgraphs.push(Subgraph { id: None, label: format!("{mark}{label}"), nodes: Vec::new(), direction: None, icon: None });
                }
                "junction" => graph.ensure_node(id, Some(String::new()), Some(NodeShape::Circle)),
                _ => graph.ensure_node(id, Some(format!("{mark}{label}")), Some(shape)),
            }
        } else if let Some((left, right)) = line.split_once("--") {
            let ((from, a), (to, b)) = (end(left), end(right));
            graph.ensure_node(from, None, None);
            graph.ensure_node(to, None, None);
            links.push((from, to, [a, b], (left.trim_end().ends_with('<'), right.trim_start().starts_with('>'))));
        }
    }
    let mut ids: Vec<&str> = graph.nodes.keys().map(String::as_str).collect();
    ids.sort_by_key(|id| graph.node_order.get(*id).copied().unwrap_or(usize::MAX));
    if ids.is_empty() {
        return None;
    }
    let (mut cells, mut taken) = (HashMap::new(), HashMap::new());
    for &start in &ids {
        if cells.contains_key(start) {
            continue;
        }
        let right = taken.keys().map(|c: &Cell| c.0).max().map_or(0, |x| x + 2);
        put(start, (right, 0), (0, 1), &mut cells, &mut taken);
        let mut queue = VecDeque::from([start]);
        while let Some(u) = queue.pop_front() {
            for &(from, to, sides, _) in &links {
                let (v, side) = if from == u { (to, sides[0]) } else if to == u { (from, sides[1]) } else { continue };
                if cells.contains_key(v) {
                    continue;
                }
                let (here, d) = (cells[u], step(side));
                put(v, (here.0 + d.0, here.1 + d.1), d, &mut cells, &mut taken);
                queue.push_back(v);
            }
        }
    }
    let sizes: HashMap<&str, _> = ids.iter().map(|&id| (id, measure(graph.nodes[id].shape, &graph.nodes[id].label))).collect();
    let (xs, ys): (BTreeSet<i32>, BTreeSet<i32>) = (cells.values().map(|c| c.0).collect(), cells.values().map(|c| c.1).collect());
    let span = |axis: &BTreeSet<i32>, of: &dyn Fn(&str) -> (i32, f64), gap: f64| {
        let wide = |k: i32| ids.iter().map(|&id| of(id)).filter(|(c, _)| *c == k).map(|(_, w)| w).fold(0.0, f64::max);
        let mut at = 0.0;
        let mut out = HashMap::new();
        for &k in axis {
            out.insert(k, (at, wide(k)));
            at += wide(k) + gap;
        }
        out
    };
    let cols = span(&xs, &|id| (cells[id].0, sizes[id].1.0), ACROSS);
    let rows = span(&ys, &|id| (cells[id].1, sizes[id].1.1), DOWN);
    let rect = |id: &str| {
        let ((x, w), (y, h), (cw, ch)) = (cols[&cells[id].0], rows[&cells[id].1], sizes[id].1);
        Rect::from_center_size((x + w / 2.0, y + h / 2.0), (cw, ch))
    };
    let mut scene = Scene::default();
    for &id in &ids {
        let at = parents.get(id).copied();
        let mut group = at;
        while let Some(g) = group {
            if let Some(&k) = groups.get(g) {
                graph.subgraphs[k].nodes.push(id.to_owned());
            }
            group = parents.get(g).copied();
        }
        scene.nodes.insert(id.to_owned(), Placed { rect: rect(id), shape: graph.nodes[id].shape, label: sizes[id].0.clone(), hidden: false });
    }
    let live: Vec<usize> = (0..graph.subgraphs.len()).filter(|&k| !graph.subgraphs[k].nodes.is_empty()).collect();
    let mut order = live.clone();
    order.sort_by_key(|&k| graph.subgraphs[k].nodes.len());
    let mut boxes: HashMap<usize, Rect> = HashMap::new();
    for &k in &order {
        let members = &graph.subgraphs[k].nodes;
        let inside = |o: usize| graph.subgraphs[o].nodes.len() < members.len() && graph.subgraphs[o].nodes.iter().all(|n| members.contains(n));
        let content = members.iter().map(|n| rect(n)).chain(boxes.iter().filter(|(o, _)| inside(**o)).map(|(_, r)| *r)).reduce(|a, b| a.union(b)).unwrap_or_default();
        boxes.insert(k, Rect::new(content.x0 - PAD, content.y0 - PAD - LINE, content.x1 + PAD, content.y1 + PAD));
    }
    for &k in &live {
        scene.frames.push(Frame { rect: boxes[&k], title: measure(NodeShape::Rectangle, &graph.subgraphs[k].label).0 });
    }
    for &(from, to, sides, arrows) in &links {
        let node = |id: &str| Node { rect: rect(id), shape: graph.nodes[id].shape };
        scene.wires.push(Wire::between(edge(from, to, None, arrows), None, node(from), node(to), sides));
    }
    Some((graph, scene))
}

pub(super) fn draw(lines: &[&str], canvas: &mut Canvas) {
    if let Some((graph, scene)) = scene(lines) {
        paint(&graph, &scene, &[], canvas);
    }
}
