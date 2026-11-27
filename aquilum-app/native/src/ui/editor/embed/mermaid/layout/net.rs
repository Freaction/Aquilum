use std::collections::HashMap;

use mermaid_rs_renderer::{Direction, Edge, Graph, StateNotePosition};

use super::super::arrow;
use super::super::canvas::Text;
use super::size;
use crate::ui::tokens::size::{GAP_MD, GAP_XL};

pub(super) const PAD: f64 = 20.0;

#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub(super) enum Kind {
    Node,
    Dummy,
    Label,
    Top,
    Bottom,
    Left,
    Right,
    Note,
}

pub(super) struct Item {
    pub kind: Kind,
    pub key: usize,
    pub w: f64,
    pub h: f64,
    pub owner: Option<usize>,
    pub rank: i32,
    pub room: f64,
    pub x: f64,
    pub y: f64,
}

pub(super) struct Group {
    pub index: usize,
    pub parent: Option<usize>,
    pub top: usize,
    pub bottom: usize,
    pub title: Text,
}

pub(super) struct Wire {
    pub edge: usize,
    pub from: usize,
    pub to: usize,
    pub label: Option<(Text, (f64, f64))>,
    pub path: Vec<usize>,
}

pub(super) struct Net {
    pub items: Vec<Item>,
    pub groups: Vec<Group>,
    pub links: Vec<(usize, usize, i32, f64)>,
    pub wires: Vec<Wire>,
    pub notes: Vec<(usize, usize, bool)>,
    pub nodes: Vec<(String, Text)>,
    pub across: bool,
}

impl Net {
    pub fn add(&mut self, kind: Kind, key: usize, (w, h): (f64, f64), owner: Option<usize>) -> usize {
        let (w, h) = if self.across { (h, w) } else { (w, h) };
        self.items.push(Item { kind, key, w, h, owner, rank: 0, room: 0.0, x: 0.0, y: 0.0 });
        self.items.len() - 1
    }

    pub fn ancestors(&self, mut group: Option<usize>) -> Vec<usize> {
        let mut chain = Vec::new();
        while let Some(g) = group {
            chain.push(g);
            group = self.groups[g].parent;
        }
        chain.reverse();
        chain
    }

    fn attach(&self, item: usize) -> Option<usize> {
        match self.items[item].kind {
            Kind::Top | Kind::Bottom => self.groups[self.items[item].key].parent,
            _ => self.items[item].owner,
        }
    }

    pub fn crossing(&self, a: usize, b: usize) -> (Option<usize>, [usize; 2]) {
        let (a, b) = (self.ancestors(self.attach(a)), self.ancestors(self.attach(b)));
        let shared = a.iter().zip(&b).take_while(|(x, y)| x == y).count();
        (shared.checked_sub(1).map(|i| a[i]), [a.len() - shared, b.len() - shared])
    }

    pub fn frame(&self, g: usize) -> (f64, f64, f64, f64) {
        let sides = |kind: Kind| self.items.iter().filter(move |i| i.kind == kind && i.key == g).map(|i| i.x);
        let (top, bottom) = (&self.items[self.groups[g].top], &self.items[self.groups[g].bottom]);
        (sides(Kind::Left).fold(f64::MAX, f64::min), top.y - top.h / 2.0, sides(Kind::Right).fold(f64::MIN, f64::max), bottom.y + bottom.h / 2.0)
    }

    pub fn twins(&self) -> Vec<bool> {
        let ends = |w: &Wire| (w.from.min(w.to), w.from.max(w.to));
        self.wires.iter().map(|w| self.wires.iter().filter(|o| ends(o) == ends(w)).count() > 1).collect()
    }

    pub fn title(&self, g: usize) -> (f64, f64) {
        let title = &self.groups[g].title;
        (title.width, title.height())
    }
}

fn groups(graph: &Graph) -> (Vec<usize>, Vec<Option<usize>>) {
    let sets: Vec<&Vec<String>> = graph.subgraphs.iter().map(|s| &s.nodes).collect();
    let live: Vec<usize> = (0..sets.len()).filter(|&i| !sets[i].is_empty()).collect();
    let holds = |a: usize, b: usize| a != b && sets[b].iter().all(|n| sets[a].contains(n)) && (sets[a].len() > sets[b].len() || a < b);
    let parent = |b: usize| live.iter().copied().filter(|&a| holds(a, b)).min_by_key(|&a| sets[a].len());
    let parents = live.iter().map(|&b| parent(b).and_then(|a| live.iter().position(|&s| s == a))).collect();
    (live, parents)
}

pub(super) fn build(graph: &Graph, loops: &[Edge]) -> Net {
    let across = matches!(graph.direction, Direction::LeftRight | Direction::RightLeft);
    let flipped = graph.direction == Direction::BottomTop;
    let mut net = Net { items: Vec::new(), groups: Vec::new(), links: Vec::new(), wires: Vec::new(), notes: Vec::new(), nodes: Vec::new(), across };
    let (live, parents) = groups(graph);
    let owner = |id: &str| live.iter().enumerate().filter(|(_, s)| graph.subgraphs[**s].nodes.iter().any(|n| n == id)).min_by_key(|(_, s)| graph.subgraphs[**s].nodes.len()).map(|(g, _)| g);
    let mut named: HashMap<&str, usize> = HashMap::new();
    for (g, &sub) in live.iter().enumerate() {
        let title = size::text(&graph.subgraphs[sub].label);
        let (tw, th) = (title.width, title.height());
        let band = |on: bool| if on && !across { (tw, th + PAD) } else { (0.0, PAD) };
        let top = net.add(Kind::Top, g, band(!flipped), Some(g));
        let bottom = net.add(Kind::Bottom, g, band(flipped), Some(g));
        net.groups.push(Group { index: sub, parent: parents[g], top, bottom, title });
        if let Some(id) = &graph.subgraphs[sub].id {
            named.insert(id.as_str(), g);
        }
    }
    let mut ids: Vec<&String> = graph.nodes.keys().filter(|id| !named.contains_key(id.as_str())).collect();
    ids.sort_by_key(|id| graph.node_order.get(*id).copied().unwrap_or(usize::MAX));
    let mut items: HashMap<&str, usize> = HashMap::new();
    for id in ids {
        let node = &graph.nodes[id];
        let label = size::text(&node.label);
        let at = net.add(Kind::Node, net.nodes.len(), size::node(node.shape, &label, across), owner(id));
        net.nodes.push((id.clone(), label));
        net.items[at].room = loops.iter().filter(|e| &e.from == id).map(|e| arrow::room(e, across)).fold(0.0, f64::max);
        items.insert(id.as_str(), at);
        if let Some(g) = net.items[at].owner {
            net.links.extend([(net.groups[g].top, at, 1, 0.0), (at, net.groups[g].bottom, 1, 0.0)]);
        }
    }
    for g in 0..net.groups.len() {
        if let Some(p) = net.groups[g].parent {
            net.links.extend([(net.groups[p].top, net.groups[g].top, 1, 0.0), (net.groups[g].bottom, net.groups[p].bottom, 1, 0.0)]);
        }
    }
    let resolve = |net: &Net, id: &str, source: bool| items.get(id).copied().or_else(|| named.get(id).map(|&g| if source { net.groups[g].bottom } else { net.groups[g].top }));
    for (edge, e) in graph.edges.iter().enumerate() {
        let (Some(from), Some(to)) = (resolve(&net, &e.from, true), resolve(&net, &e.to, false)) else { continue };
        if from == to {
            continue;
        }
        let label = e.label.as_deref().filter(|l| !l.trim().is_empty()).map(size::chip);
        let (_, [out, into]) = net.crossing(from, to);
        net.links.push((from, to, 2 + (out + into) as i32, 1.0));
        net.wires.push(Wire { edge, from, to, label, path: Vec::new() });
    }
    for (n, note) in graph.state_notes.iter().enumerate() {
        let Some(&target) = items.get(note.target.as_str()) else { continue };
        let text = size::text(&note.label);
        let dims = (text.width + GAP_XL * 2.0, text.height() + GAP_MD * 2.0);
        let at = net.add(Kind::Note, n, dims, net.items[target].owner);
        net.notes.push((at, target, note.position == StateNotePosition::LeftOf));
    }
    net
}
