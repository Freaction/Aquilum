use std::cmp::Ordering;
use std::collections::HashMap;

use super::net::{Kind, Net};

const PASSES: usize = 12;
const GROUP: usize = 1 << 30;

pub(super) struct Lanes {
    pub up: Vec<Vec<usize>>,
    pub down: Vec<Vec<usize>>,
    twin: Vec<Option<usize>>,
}

pub(super) fn lanes(net: &Net) -> Lanes {
    let n = net.items.len();
    let (mut up, mut down) = (vec![Vec::new(); n], vec![Vec::new(); n]);
    for pair in net.wires.iter().flat_map(|w| w.path.windows(2)) {
        down[pair[0]].push(pair[1]);
        up[pair[1]].push(pair[0]);
    }
    let mut twin = vec![None; n];
    for (w, paired) in net.wires.iter().zip(net.twins()) {
        for &i in w.path[1..w.path.len() - 1].iter().filter(|_| paired) {
            twin[i] = Some(w.from.min(w.to));
        }
    }
    Lanes { up, down, twin }
}

pub(super) fn layers(net: &Net) -> Vec<Vec<usize>> {
    let height = net.items.iter().map(|i| i.rank as usize + 1).max().unwrap_or(0);
    let mut layers = vec![Vec::new(); height];
    for (i, item) in net.items.iter().enumerate() {
        layers[item.rank as usize].push(i);
    }
    layers
}

fn compare(a: &[(f64, usize)], b: &[(f64, usize)]) -> Ordering {
    a.iter().zip(b).map(|(x, y)| x.0.total_cmp(&y.0).then(x.1.cmp(&y.1))).find(|o| o.is_ne()).unwrap_or(a.len().cmp(&b.len()))
}

fn sort(net: &Net, lanes: &Lanes, layer: &mut [usize], bc: &[f64]) {
    let border = |i: usize| matches!(net.items[i].kind, Kind::Left | Kind::Right);
    let mut sums: HashMap<usize, (f64, f64)> = HashMap::new();
    let mut known: Vec<usize> = Vec::new();
    for pass in [false, true] {
        for &i in layer.iter().filter(|&&i| border(i) == pass) {
            for g in net.ancestors(net.items[i].owner).into_iter().filter(|g| !known.contains(g)) {
                let entry = sums.entry(g).or_default();
                *entry = (entry.0 + bc[i], entry.1 + 1.0);
            }
        }
        known = sums.keys().copied().collect();
    }
    let key = |i: usize| {
        let item = &net.items[i];
        let mut key: Vec<(f64, usize)> = net.ancestors(item.owner).into_iter().map(|g| (sums[&g].0 / sums[&g].1, GROUP + g)).collect();
        let own = match item.kind {
            Kind::Left => f64::NEG_INFINITY,
            Kind::Right => f64::INFINITY,
            _ => bc[i],
        };
        key.push(match lanes.twin[i] {
            Some(t) => {
                let pair: Vec<f64> = layer.iter().filter(|&&j| lanes.twin[j] == Some(t)).map(|&j| bc[j]).collect();
                (pair.iter().sum::<f64>() / pair.len() as f64, usize::MAX - t)
            }
            None => (own, i),
        });
        key
    };
    let mut keyed: Vec<(Vec<(f64, usize)>, usize)> = layer.iter().map(|&i| (key(i), i)).collect();
    keyed.sort_by(|a, b| compare(&a.0, &b.0));
    layer.iter_mut().zip(keyed).for_each(|(slot, (_, i))| *slot = i);
}

fn barycenters(net: &Net, layer: &[usize], near: &[Vec<usize>], pos: &[f64], bc: &mut [f64]) {
    for &v in layer {
        bc[v] = if near[v].is_empty() { pos[v] } else { near[v].iter().map(|&u| pos[u]).sum::<f64>() / near[v].len() as f64 };
    }
    for &(item, target, left) in net.notes.iter().filter(|n| layer.contains(&n.0)) {
        bc[item] = bc[target] + if left { -0.5 } else { 0.5 };
    }
}

fn crossings(layers: &[Vec<usize>], down: &[Vec<usize>], pos: &[f64]) -> usize {
    let mut total = 0;
    for layer in layers {
        let segments: Vec<(f64, f64)> = layer.iter().flat_map(|&u| down[u].iter().map(move |&v| (pos[u], pos[v]))).collect();
        for (i, a) in segments.iter().enumerate() {
            total += segments[i + 1..].iter().filter(|b| (a.0 - b.0) * (a.1 - b.1) < 0.0).count();
        }
    }
    total
}

fn index(layers: &[Vec<usize>], pos: &mut [f64]) {
    for layer in layers {
        layer.iter().enumerate().for_each(|(i, &v)| pos[v] = i as f64);
    }
}

pub(super) fn arrange(net: &Net, lanes: &Lanes, layers: &mut [Vec<usize>]) {
    let mut pos = vec![0.0; net.items.len()];
    index(layers, &mut pos);
    for layer in layers.iter_mut() {
        let bc = pos.clone();
        sort(net, lanes, layer, &bc);
    }
    index(layers, &mut pos);
    let mut bc = pos.clone();
    let (mut best, mut least) = (layers.to_vec(), crossings(layers, &lanes.down, &pos));
    for pass in 0..PASSES {
        let down = pass % 2 == 0;
        let ranks: Vec<usize> = if down { (1..layers.len()).collect() } else { (0..layers.len().saturating_sub(1)).rev().collect() };
        let near = if down { &lanes.up } else { &lanes.down };
        for r in ranks {
            barycenters(net, &layers[r], near, &pos, &mut bc);
            sort(net, lanes, &mut layers[r], &bc);
            layers[r].iter().enumerate().for_each(|(i, &v)| pos[v] = i as f64);
        }
        let count = crossings(layers, &lanes.down, &pos);
        if count < least {
            (best, least) = (layers.to_vec(), count);
        }
    }
    layers.clone_from_slice(&best);
}
