use masonry::kurbo::{ParamCurve, Point, Vec2};

use super::super::arrow::{bow, find};
use super::net::{Kind, Net};
use crate::ui::tokens::size::GAP_MD;

const STRAIGHT: f64 = 1.0;
const FIRM: f64 = 256.0;

#[derive(Clone, Copy)]
struct Step {
    wire: usize,
    at: f64,
    shift: f64,
}

pub(super) struct Bend {
    steps: Vec<Option<Step>>,
    ends: Vec<Vec<(usize, usize)>>,
    base: Vec<f64>,
    off: Vec<[f64; 2]>,
    twin: Vec<bool>,
    firm: f64,
}

fn fraction(y0: f64, y1: f64, y: f64) -> f64 {
    let curve = bow(Point::new(0.0, y0), Vec2::new(0.0, 1.0), Point::new(1.0, y1), Vec2::new(0.0, -1.0));
    curve.eval(find(|t| curve.eval(t).y - y)).x
}

fn inner(net: &Net, w: usize) -> &[usize] {
    let path = &net.wires[w].path;
    &path[1..path.len() - 1]
}

impl Bend {
    pub fn new(net: &Net) -> Bend {
        let mut ends = vec![Vec::new(); net.items.len()];
        for (w, wire) in net.wires.iter().enumerate() {
            ends[wire.path[0]].push((w, 0));
            ends[wire.path[wire.path.len() - 1]].push((w, 1));
        }
        let base = net.items.iter().map(|i| i.w).collect();
        Bend { steps: vec![None; net.items.len()], ends, base, off: vec![[0.0; 2]; net.wires.len()], twin: net.twins(), firm: 0.0 }
    }

    fn port(&self, net: &Net, w: usize, end: usize) -> f64 {
        let path = &net.wires[w].path;
        net.items[if end == 0 { path[0] } else { path[path.len() - 1] }].x + self.off[w][end]
    }

    pub fn refresh(&mut self, net: &mut Net, bands: &[f64], ports: &[[Point; 2]], pass: usize) {
        self.firm = (4.0 * 2f64.powf(pass as f64 / 8.0)).min(FIRM);
        for (w, [a, b]) in ports.iter().enumerate() {
            let path = &net.wires[w].path;
            self.off[w] = [a.x - net.items[path[0]].x, b.x - net.items[path[path.len() - 1]].x];
            let span = (b.x - a.x).abs();
            for i in inner(net, w).to_vec() {
                let item = &net.items[i];
                let half = bands[item.rank as usize] / 2.0;
                let [lo, mid, hi] = [item.y - half, item.y, item.y + half].map(|y| fraction(a.y, b.y, y));
                let beside = if self.twin[w] && item.kind == Kind::Label { self.off[w][0].signum() } else { 0.0 };
                let (at, width, shift) = match item.kind {
                    _ if beside != 0.0 => (mid, self.base[i], beside * (self.base[i] / 2.0 + GAP_MD + span * (hi - lo) / 2.0)),
                    Kind::Label => (mid, self.base[i].max(2.0 * span * (mid - lo).max(hi - mid)), 0.0),
                    _ => ((lo + hi) / 2.0, span * (hi - lo), 0.0),
                };
                self.steps[i] = Some(Step { wire: w, at, shift });
                net.items[i].w = width;
            }
        }
    }

    pub fn along(&self, net: &Net, v: usize) -> Option<(f64, f64)> {
        let step = self.steps[v]?;
        let (p, q) = (self.port(net, step.wire, 0), self.port(net, step.wire, 1));
        Some((p + (q - p) * step.at + step.shift, self.firm))
    }

    pub fn pull(&self, net: &Net, v: usize) -> Option<(f64, f64)> {
        let (mut sum, mut weight) = (0.0, 0.0);
        for &(w, end) in &self.ends[v] {
            let other = self.port(net, w, 1 - end);
            let (mut a, mut rhs) = (STRAIGHT, STRAIGHT * other);
            for &i in inner(net, w) {
                let Some(step) = self.steps[i] else { continue };
                let c = if end == 0 { 1.0 - step.at } else { step.at };
                a += self.firm * c * c;
                rhs += self.firm * c * (net.items[i].x - step.shift - (1.0 - c) * other);
            }
            sum += rhs - a * self.off[w][end];
            weight += a;
        }
        (weight > 0.0).then(|| (sum / weight, weight))
    }
}
