use std::collections::VecDeque;

use super::cycles::{self, Link, adjacency};
use super::net::{Kind, Net};

fn topo(n: usize, links: &[Link], outs: &[Vec<usize>]) -> Vec<usize> {
    let mut indeg = vec![0usize; n];
    links.iter().for_each(|l| indeg[l.1] += 1);
    let mut queue: VecDeque<usize> = (0..n).filter(|&v| indeg[v] == 0).collect();
    let mut order = Vec::with_capacity(n);
    while let Some(v) = queue.pop_front() {
        order.push(v);
        for &k in &outs[v] {
            indeg[links[k].1] -= 1;
            if indeg[links[k].1] == 0 {
                queue.push_back(links[k].1);
            }
        }
    }
    order
}

pub(super) fn assign(net: &mut Net) {
    let n = net.items.len();
    let mut links = net.links.clone();
    cycles::untangle(n, &mut links);
    let (ins, outs) = adjacency(n, &links);
    let order = topo(n, &links, &outs);
    let mut rank = vec![0i32; n];
    for &v in &order {
        rank[v] = ins[v].iter().map(|&k| rank[links[k].0] + links[k].2).max().unwrap_or(0);
    }
    let weight = |ks: &[usize]| ks.iter().map(|&k| links[k].3).sum::<f64>();
    for &v in order.iter().rev() {
        let Some(down) = outs[v].iter().map(|&k| rank[links[k].1] - links[k].2).min() else { continue };
        let pull = ins[v].is_empty() || net.items[v].kind == Kind::Top || weight(&ins[v]) < weight(&outs[v]);
        if down > rank[v] && pull {
            rank[v] = down;
        }
    }
    let low = rank.iter().copied().min().unwrap_or(0);
    for (item, r) in net.items.iter_mut().zip(rank) {
        item.rank = r - low;
    }
}

fn facing(net: &Net, end: usize, other: usize) -> usize {
    if !matches!(net.items[end].kind, Kind::Top | Kind::Bottom) {
        return end;
    }
    let group = &net.groups[net.items[end].key];
    let rank = net.items[other].rank;
    if rank < net.items[group.top].rank { group.top } else if rank > net.items[group.bottom].rank { group.bottom } else { end }
}

pub(super) fn chain(net: &mut Net) {
    for n in 0..net.notes.len() {
        let (item, target, _) = net.notes[n];
        net.items[item].rank = net.items[target].rank;
    }
    for w in 0..net.wires.len() {
        let from = facing(net, net.wires[w].from, net.wires[w].to);
        let to = facing(net, net.wires[w].to, from);
        (net.wires[w].from, net.wires[w].to) = (from, to);
        let (lo, hi) = if net.items[from].rank <= net.items[to].rank { (from, to) } else { (to, from) };
        let (owner, [out, into]) = net.crossing(lo, hi);
        let (r0, r1) = (net.items[lo].rank, net.items[hi].rank);
        let mid = (r0 + out as i32 + r1 - into as i32) / 2;
        let mut path = vec![lo];
        for r in r0 + 1..r1 {
            let label = net.wires[w].label.as_ref().filter(|_| r == mid).map(|l| l.1);
            let item = match label {
                Some(dims) => net.add(Kind::Label, w, dims, owner),
                None => net.add(Kind::Dummy, w, (0.0, 0.0), owner),
            };
            net.items[item].rank = r;
            path.push(item);
        }
        path.push(hi);
        net.wires[w].path = path;
    }
    for g in 0..net.groups.len() {
        let (top, bottom) = (net.items[net.groups[g].top].rank, net.items[net.groups[g].bottom].rank);
        for r in top..=bottom {
            for kind in [Kind::Left, Kind::Right] {
                let item = net.add(kind, g, (0.0, 0.0), Some(g));
                net.items[item].rank = r;
            }
        }
    }
}
