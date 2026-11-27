use std::collections::HashMap;

use masonry::kurbo::Rect;
use mermaid_rs_renderer::Graph;

use super::super::scene::{Placed, Scene, Wire};
use super::super::shape::{Node, Side};
use super::size;

const ACROSS: f64 = 72.0;
const DOWN: f64 = 16.0;

struct Tree<'a> {
    kids: HashMap<&'a str, Vec<&'a str>>,
    dims: HashMap<&'a str, (f64, f64)>,
    depth: HashMap<&'a str, usize>,
    left: HashMap<&'a str, bool>,
    span: HashMap<&'a str, f64>,
    mid: HashMap<&'a str, f64>,
}

impl<'a> Tree<'a> {
    fn kids(&self, v: &str) -> Vec<&'a str> {
        self.kids.get(v).cloned().unwrap_or_default()
    }

    fn stack(&self, kids: &[&'a str]) -> f64 {
        kids.iter().map(|k| self.span[k]).sum::<f64>() + DOWN * kids.len().saturating_sub(1) as f64
    }

    fn measure(&mut self, v: &'a str, depth: usize, left: bool) -> f64 {
        self.depth.insert(v, depth);
        self.left.insert(v, left);
        let kids = self.kids(v);
        kids.iter().for_each(|&k| _ = self.measure(k, depth + 1, left));
        let span = self.stack(&kids).max(self.dims[v].1);
        self.span.insert(v, span);
        span
    }

    fn place(&mut self, v: &'a str, top: f64) {
        let kids = self.kids(v);
        let mut y = top + (self.span[v] - self.stack(&kids)).max(0.0) / 2.0;
        for &k in &kids {
            self.place(k, y);
            y += self.span[k] + DOWN;
        }
        let centre = match (kids.first(), kids.last()) {
            (Some(a), Some(b)) => (self.mid[a] + self.mid[b]) / 2.0,
            _ => top + self.span[v] / 2.0,
        };
        self.mid.insert(v, centre);
    }
}

pub(in super::super) fn tree(graph: &Graph) -> Scene {
    let mut kids: HashMap<&str, Vec<&str>> = HashMap::new();
    for e in &graph.edges {
        kids.entry(e.from.as_str()).or_default().push(e.to.as_str());
    }
    let dims = graph.nodes.iter().map(|(id, n)| (id.as_str(), size::node(n.shape, &size::text(&n.label), false))).collect();
    let root = graph.mindmap.root_id.as_deref().or_else(|| graph.nodes.keys().map(String::as_str).find(|id| graph.edges.iter().all(|e| e.to != *id)));
    let mut tree = Tree { kids, dims, depth: HashMap::new(), left: HashMap::new(), span: HashMap::new(), mid: HashMap::new() };
    let mut scene = Scene::default();
    if let Some(root) = root.filter(|r| tree.dims.contains_key(r)) {
        tree.depth.insert(root, 0);
        tree.left.insert(root, false);
        tree.mid.insert(root, 0.0);
        let mut sides: [Vec<&str>; 2] = [Vec::new(), Vec::new()];
        let mut load = [0.0f64; 2];
        for k in tree.kids(root) {
            let s = usize::from(load[1] < load[0]);
            load[s] += tree.measure(k, 1, s == 1) + DOWN;
            sides[s].push(k);
        }
        for side in &sides {
            let mut y = -tree.stack(side) / 2.0;
            for &k in side {
                tree.place(k, y);
                y += tree.span[k] + DOWN;
            }
        }
        let levels = tree.depth.values().max().map_or(0, |d| d + 1);
        let mut columns = vec![[0.0f64; 2]; levels];
        for (id, &d) in &tree.depth {
            let s = usize::from(tree.left[id]);
            columns[d][s] = columns[d][s].max(tree.dims[id].0);
        }
        let half = tree.dims[root].0 / 2.0;
        let inner: Vec<[f64; 2]> = (0..levels).map(|d| [0, 1].map(|s| half + (1..d).map(|i| columns[i][s] + ACROSS).sum::<f64>() + ACROSS)).collect();
        let rect = |id: &str| {
            let ((w, h), d, y) = (tree.dims[id], tree.depth[id], tree.mid[id]);
            let x0 = match (d, tree.left[id]) {
                (0, _) => -w / 2.0,
                (_, false) => inner[d][0],
                (_, true) => -inner[d][1] - w,
            };
            Rect::new(x0, y - h / 2.0, x0 + w, y + h / 2.0)
        };
        for &id in tree.depth.keys() {
            let n = &graph.nodes[id];
            scene.nodes.insert(id.to_owned(), Placed { rect: rect(id), shape: n.shape, label: size::text(&n.label), hidden: false });
        }
        for e in graph.edges.iter().filter(|e| tree.depth.contains_key(e.from.as_str()) && tree.depth.contains_key(e.to.as_str())) {
            let sides = if tree.left[e.to.as_str()] { [Side::Left, Side::Right] } else { [Side::Right, Side::Left] };
            let node = |id: &str| Node { rect: rect(id), shape: graph.nodes[id].shape };
            scene.wires.push(Wire::between(e.clone(), None, node(&e.from), node(&e.to), sides));
        }
    }
    scene
}
