#[cfg(test)]
mod tests;

use std::collections::HashSet;

use masonry::kurbo::{Point, Rect};
use mermaid_rs_renderer::{Edge, Graph};

use super::scene::Scene;
use super::{arrow, built, graph, laid};

const BESIDE: f64 = 24.0;

pub(super) struct Measure {
    pub faults: Vec<String>,
    pub crossings: usize,
    pub bends: usize,
}

fn clip(a: Point, b: Point, r: Rect) -> bool {
    let (mut t0, mut t1) = (0.0f64, 1.0f64);
    let d = b - a;
    for (p, q) in [(-d.x, a.x - r.x0), (d.x, r.x1 - a.x), (-d.y, a.y - r.y0), (d.y, r.y1 - a.y)] {
        if p.abs() < 1e-12 {
            if q < 0.0 {
                return false;
            }
            continue;
        }
        let t = q / p;
        if p < 0.0 { t0 = t0.max(t) } else { t1 = t1.min(t) }
        if t0 > t1 {
            return false;
        }
    }
    true
}

pub(super) fn hits(path: &[Point], r: Rect) -> bool {
    let r = r.inflate(-2.0, -2.0);
    r.width() > 0.0 && r.height() > 0.0 && path.windows(2).any(|w| clip(w[0], w[1], r))
}

fn cross(a: Point, b: Point, c: Point, d: Point) -> bool {
    let side = |p: Point, q: Point, r: Point| (q - p).cross(r - p);
    let (d1, d2, d3, d4) = (side(c, d, a), side(c, d, b), side(a, b, c), side(a, b, d));
    d1 * d2 < 0.0 && d3 * d4 < 0.0
}

fn crossings(a: &[Point], b: &[Point]) -> usize {
    a.windows(2).map(|s| b.windows(2).filter(|t| cross(s[0], s[1], t[0], t[1])).count()).sum()
}

fn bends(path: &[Point]) -> usize {
    let turns: Vec<f64> = path.windows(3).map(|w| (w[1] - w[0]).cross(w[2] - w[1])).filter(|t| t.abs() > 0.5).collect();
    turns.windows(2).filter(|w| w[0].signum() != w[1].signum()).count()
}

fn near(path: &[Point], r: Rect) -> Vec<Point> {
    let r = r.inflate(6.0, 6.0);
    path.iter().copied().filter(|p| !r.contains(*p)).collect()
}

pub(super) fn measure(source: &str) -> Option<Measure> {
    if let Some(laid) = laid(source) {
        return Some(faults(&laid.graph, laid.scene.as_ref()?, &laid.loops));
    }
    let (graph, scene) = built(source)?;
    Some(faults(&graph, &scene, &[]))
}

fn faults(graph: &Graph, scene: &Scene, loops: &[Edge]) -> Measure {
    let (nodes, wires, vertical) = graph::prepare(graph, scene, loops);
    let lines = arrow::lines(&wires, &nodes, vertical);
    let edges: Vec<&Edge> = wires.iter().map(|w| &w.edge).collect();
    let visible: Vec<(&str, Rect)> = scene.nodes.iter().filter(|(_, n)| !n.hidden).map(|(id, n)| (id.as_str(), n.rect)).collect();
    let mut faults = Vec::new();
    for (i, (a, ra)) in visible.iter().enumerate() {
        for (b, rb) in &visible[i + 1..] {
            if ra.intersect(*rb).area() > 1.0 {
                faults.push(format!("узлы «{a}» и «{b}» накладываются"));
            }
        }
    }
    let subs = graph.subgraphs.iter().filter(|s| !s.nodes.is_empty());
    let groups: Vec<(Rect, HashSet<&str>, &str)> = scene.frames.iter().zip(subs).map(|(f, s)| (f.rect, s.nodes.iter().map(String::as_str).collect(), s.label.as_str())).collect();
    for (i, (ra, ma, a)) in groups.iter().enumerate() {
        for (rb, mb, b) in &groups[i + 1..] {
            let (inner, outer) = if mb.is_subset(ma) { (*rb, *ra) } else { (*ra, *rb) };
            if ma.is_subset(mb) || mb.is_subset(ma) {
                if !outer.inflate(1.0, 1.0).contains_rect(inner) {
                    faults.push(format!("рамка «{a}»/«{b}» не вложена в свою"));
                }
            } else if ra.intersect(*rb).area() > 1.0 {
                faults.push(format!("рамки «{a}» и «{b}» накладываются"));
            }
        }
        for (id, r) in &visible {
            let member = ma.contains(id);
            if member && !ra.inflate(1.0, 1.0).contains_rect(*r) {
                faults.push(format!("«{id}» выходит из своей рамки «{a}»"));
            } else if !member && ra.intersect(*r).area() > 1.0 {
                faults.push(format!("«{id}» залез в чужую рамку «{a}»"));
            }
        }
    }
    let notes: Vec<Rect> = scene.notes.iter().map(|(r, _)| *r).collect();
    let chips: Vec<Option<Rect>> = wires.iter().zip(&lines).map(|(w, l)| l.as_ref().and_then(|l| arrow::chip(w, l))).collect();
    let marks: Vec<Vec<Rect>> = wires.iter().zip(&lines).map(|(w, l)| l.as_ref().map_or(Vec::new(), |l| arrow::marks(w, l, &nodes).iter().map(|m| m.rect()).collect())).collect();
    for (k, rects) in marks.iter().enumerate() {
        let name = format!("{}→{}", edges[k].from, edges[k].to);
        for r in rects {
            let mut near: Vec<String> = visible.iter().filter(|(_, n)| n.intersect(*r).area() > 1.0).map(|(id, _)| format!("«{id}»")).collect();
            near.extend(chips.iter().flatten().filter(|c| c.intersect(*r).area() > 1.0).map(|_| "подпись".to_owned()));
            near.extend(marks.iter().enumerate().filter(|(j, _)| *j > k).flat_map(|(_, o)| o).filter(|o| o.intersect(*r).area() > 1.0).map(|_| "кратность".to_owned()));
            near.extend(lines.iter().enumerate().filter(|(j, l)| *j != k && l.as_ref().is_some_and(|l| hits(&l.spine, *r))).map(|(j, _)| format!("стрелка {}→{}", edges[j].from, edges[j].to)));
            for what in near {
                faults.push(format!("кратность у {name} лежит на: {what}"));
            }
        }
    }
    let (mut crossed, mut bent) = (0, 0);
    for (k, (edge, line)) in edges.iter().zip(&lines).enumerate() {
        let Some(line) = line else { continue };
        let name = format!("{}→{}", edge.from, edge.to);
        bent += bends(&line.spine);
        let ends: Vec<Rect> = [&edge.from, &edge.to].iter().filter_map(|id| nodes.get(id.as_str()).map(|n| n.rect)).collect();
        let free = ends.iter().fold(line.spine.clone(), |path, r| near(&path, *r));
        for (id, r) in visible.iter().filter(|(id, _)| *id != edge.from && *id != edge.to) {
            if hits(&free, *r) {
                faults.push(format!("стрелка {name} проходит через «{id}»"));
            }
        }
        for r in &notes {
            if hits(&free, *r) {
                faults.push(format!("стрелка {name} проходит через заметку"));
            }
        }
        for (r, members, g) in &groups {
            let attached = |id: &String| members.contains(id.as_str()) || nodes.get(id.as_str()).is_some_and(|n| n.rect.inflate(1.0, 1.0).contains_rect(*r));
            if !attached(&edge.from) && !attached(&edge.to) && hits(&line.spine, *r) {
                faults.push(format!("стрелка {name} режет рамку «{g}»"));
            }
        }
        if let Some(chip) = chips[k] {
            let twin = edge.from == edge.to || edges.iter().enumerate().any(|(j, e)| j != k && ((e.from == edge.from && e.to == edge.to) || (e.from == edge.to && e.to == edge.from)));
            let reach = if edge.from == edge.to { BESIDE * 3.0 } else { BESIDE };
            let near = if twin { hits(&line.spine, chip.inflate(reach, reach)) && !hits(&line.spine, chip) } else { hits(&line.spine, chip.inflate(-chip.width() / 4.0, -chip.height() / 4.0)) };
            if !near {
                faults.push(format!("подпись {name} не на своей стрелке"));
            }
            for (id, r) in &visible {
                if chip.intersect(*r).area() > 1.0 {
                    faults.push(format!("подпись {name} лежит на «{id}»"));
                }
            }
            for (j, other) in lines.iter().enumerate().filter(|(j, _)| *j != k) {
                if other.as_ref().is_some_and(|o| hits(&o.spine, chip)) {
                    faults.push(format!("подпись {name} лежит на стрелке {}→{}", edges[j].from, edges[j].to));
                }
                if chips[j].is_some_and(|c| j > k && c.intersect(chip).area() > 1.0) {
                    faults.push(format!("подписи {name} и {}→{} накладываются", edges[j].from, edges[j].to));
                }
            }
        }
        for other in lines[k + 1..].iter().flatten() {
            crossed += crossings(&line.spine, &other.spine);
        }
    }
    Measure { faults, crossings: crossed, bends: bent }
}
