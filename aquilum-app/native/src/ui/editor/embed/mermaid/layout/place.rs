use masonry::kurbo::Point;

use super::bend::Bend;
use super::solve::{solve, spaces};
use super::net::{Item, Kind, Net, PAD};

const PASSES: usize = 160;
const OUTSIDE: f64 = 16.0;
const NOTE: f64 = 16.0;
const RIM: f64 = 1000.0;
const YIELD: f64 = 6.0;

pub(super) struct Gaps {
    pub node: f64,
    pub rank: f64,
}

struct Scope<'a> {
    gaps: &'a Gaps,
    inside: Vec<Vec<usize>>,
    beside: Vec<[Option<usize>; 2]>,
}

fn side(net: &Net, item: &Item) -> f64 {
    PAD + if net.across && item.kind == Kind::Left { net.title(item.key).1 } else { 0.0 }
}

pub(super) fn gap(net: &Net, a: usize, b: usize, gaps: &Gaps) -> f64 {
    let (x, y) = (&net.items[a], &net.items[b]);
    let spread = x.room + if x.kind == Kind::Dummy && y.kind == Kind::Dummy { 0.0 } else { (x.w + y.w) / 2.0 };
    spread + match (x.kind, y.kind) {
        (Kind::Left, _) => side(net, x),
        (_, Kind::Right) => PAD,
        (Kind::Right, _) | (_, Kind::Left) => OUTSIDE,
        (Kind::Dummy, Kind::Dummy) => 12.0,
        (Kind::Note, _) | (_, Kind::Note) => NOTE,
        (Kind::Dummy | Kind::Label, _) | (_, Kind::Dummy | Kind::Label) => 20.0,
        _ => gaps.node,
    }
}

fn bounds(net: &Net, inside: &[usize]) -> Option<(f64, f64)> {
    inside.iter().map(|&i| &net.items[i]).map(|i| (i.x - i.w / 2.0, i.x + i.w / 2.0 + i.room)).reduce(|a, b| (a.0.min(b.0), a.1.max(b.1)))
}

fn rim(net: &Net, scope: &Scope, v: usize) -> (f64, f64) {
    let item = &net.items[v];
    let right = item.kind == Kind::Right;
    let Some((lo, hi)) = bounds(net, &scope.inside[item.key]) else { return (item.x, 0.1) };
    let bound = if right { hi + PAD } else { lo - side(net, item) };
    match scope.beside[v][usize::from(right)] {
        Some(n) if matches!(net.items[n].kind, Kind::Dummy | Kind::Label) => {
            let limit = if right { net.items[n].x - gap(net, v, n, scope.gaps) } else { net.items[n].x + gap(net, n, v, scope.gaps) };
            (if right { bound.min(limit) } else { bound.max(limit) }, YIELD)
        }
        Some(n) if matches!(net.items[n].kind, Kind::Node | Kind::Note) => (bound, RIM),
        _ => (bound, YIELD),
    }
}

fn target(net: &Net, scope: &Scope, bend: &Bend, v: usize) -> (f64, f64) {
    let item = &net.items[v];
    match item.kind {
        Kind::Left | Kind::Right => rim(net, scope, v),
        Kind::Note => {
            let &(_, t, left) = net.notes.iter().find(|n| n.0 == v).unwrap_or(&(v, v, false));
            let t = &net.items[t];
            let offset = (t.w + item.w) / 2.0 + NOTE;
            (if left { t.x - offset } else { t.x + offset }, 8.0)
        }
        Kind::Dummy | Kind::Label => bend.along(net, v).unwrap_or((item.x, 0.05)),
        Kind::Top | Kind::Bottom => bounds(net, &scope.inside[item.key]).map_or((item.x, 0.1), |(l, r)| ((l + r) / 2.0, 0.5)),
        Kind::Node => bend.pull(net, v).unwrap_or((item.x, 0.05)),
    }
}

fn scope<'a>(net: &Net, layers: &[Vec<usize>], gaps: &'a Gaps) -> Scope<'a> {
    let border = |i: usize, g: usize| matches!(net.items[i].kind, Kind::Left | Kind::Right) && net.items[i].key == g;
    let inside = (0..net.groups.len()).map(|g| (0..net.items.len()).filter(|&i| net.ancestors(net.items[i].owner).contains(&g) && !border(i, g)).collect()).collect();
    let mut beside = vec![[None, None]; net.items.len()];
    for layer in layers {
        for (k, &i) in layer.iter().enumerate() {
            beside[i] = [k.checked_sub(1).map(|p| layer[p]), layer.get(k + 1).copied()];
        }
    }
    Scope { gaps, inside, beside }
}

pub(super) type Ports<'a> = &'a dyn Fn(&Net) -> Vec<[Point; 2]>;

pub(super) fn place(net: &mut Net, layers: &[Vec<usize>], gaps: &Gaps, extra: &[f64], ports: Ports, bend: &mut Bend) {
    let scope = scope(net, layers, gaps);
    let bands: Vec<f64> = layers.iter().map(|l| l.iter().map(|&i| net.items[i].h).fold(0.0, f64::max)).collect();
    let mut y = 0.0;
    for (r, layer) in layers.iter().enumerate() {
        if r > 0 {
            y += bands[r - 1] / 2.0 + gaps.rank / 2.0 + bands[r] / 2.0 + extra[r - 1] + extra[r];
        }
        let mut x = 0.0;
        for (k, &i) in layer.iter().enumerate() {
            x += if k > 0 { gap(net, layer[k - 1], i, gaps) } else { 0.0 };
            net.items[i].x = x;
            net.items[i].y = y;
        }
    }
    for pass in 0..PASSES {
        bend.refresh(net, &bands, &ports(net), pass);
        let ranks: Vec<usize> = if pass % 2 == 1 { (0..layers.len()).rev().collect() } else { (0..layers.len()).collect() };
        for r in ranks {
            let layer = &layers[r];
            let targets: Vec<(f64, f64)> = layer.iter().map(|&v| target(net, &scope, bend, v)).collect();
            let spaces = spaces(net, layer, gaps);
            for (&v, x) in layer.iter().zip(solve(&targets, &spaces)) {
                net.items[v].x = x;
            }
        }
    }
}
