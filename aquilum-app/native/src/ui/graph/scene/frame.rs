use masonry::peniko::Color;

use crate::graph::raster::{Band, Disc, Edge, Rgb, Stamp};
use crate::graph::{self, Graph};

pub(super) type Channels = [f32; 4];

pub(super) fn channels(c: Color) -> Channels {
    c.components
}

pub(super) fn rgb(c: Color) -> Rgb {
    let c = c.to_rgba8();
    [c.r, c.g, c.b]
}

fn smoothstep(e0: f64, e1: f64, x: f64) -> f64 {
    let t = ((x - e0) / (e1 - e0)).clamp(0.0, 1.0);
    t * t * (3.0 - 2.0 * t)
}

fn lerp(a: Channels, b: Channels, t: f32) -> Channels {
    [0, 1, 2, 3].map(|i| a[i] + (b[i] - a[i]) * t)
}

pub(super) fn tone(back: Channels, c: Channels, amount: f32) -> Rgb {
    let a = (c[3] * amount).clamp(0.0, 1.0);
    let ch = |i: usize| ((back[i] + (c[i] - back[i]) * a).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    [ch(0), ch(1), ch(2)]
}

pub(super) struct Frame<'a> {
    pub graph: &'a Graph,
    pub screen: &'a [[f32; 2]],
    pub reveal: &'a [f32],
    pub marks: &'a [u8],
    pub intensity: &'a [f32],
    pub width: f32,
    pub height: f32,
    pub sf: f64,
    pub scale: f64,
    pub spread: f64,
    pub center_y: f64,
    pub half_h: f64,
    pub node_size: f64,
    pub dimming: f64,
    pub edge_alpha: f64,
    pub edge_active_alpha: f64,
    pub coverage: f64,
    pub back: Channels,
    pub edge: Channels,
    pub edge_active: Channels,
    pub node: Channels,
    pub rim: Channels,
    pub cold: Channels,
    pub hot: Channels,
    pub heat: Option<(&'a [f32], graph::Range)>,
    pub node_margin: f64,
    pub everything: bool,
}

impl Frame<'_> {
    fn world_y(&self, row: f64) -> f32 {
        ((self.center_y + (self.half_h - row / self.sf) / self.scale) / self.spread) as f32
    }

    fn edge(&self, index: u32, rows: &std::ops::Range<usize>) -> Option<Edge> {
        let [a, b] = self.graph.edges[index as usize];
        let (a, b) = (a as usize, b as usize);
        let (p, q) = (self.screen[a], self.screen[b]);
        if p[1].max(q[1]) < rows.start as f32 - 2.0 || p[1].min(q[1]) > rows.end as f32 + 2.0 {
            return None;
        }
        let shown = self.reveal[a].min(self.reveal[b]);
        if shown <= 0.0 {
            return None;
        }
        let (w, h) = (self.width, self.height);
        if (p[0] < 0.0 && q[0] < 0.0) || (p[0] > w && q[0] > w) || (p[1] < 0.0 && q[1] < 0.0) || (p[1] > h && q[1] > h) {
            return None;
        }
        let (ma, mb) = (self.marks[a], self.marks[b]);
        let active = if ma.min(mb) > 0 && ma.abs_diff(mb) == 1 { f64::from(self.intensity[a].min(self.intensity[b])) } else { 0.0 };
        let dim = graph::dimmed(self.dimming, active);
        let (color, alpha) = if active > 0.0 {
            let c = lerp(self.edge, self.edge_active, active as f32);
            (c, f64::from(c[3]) * self.edge_active_alpha)
        } else {
            (self.edge, self.edge_alpha)
        };
        let to_u8 = |v: f32| (v.clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
        Some(Edge { from: p, to: q, color: [to_u8(color[0]), to_u8(color[1]), to_u8(color[2])], alpha: (alpha * self.coverage.max(active) * dim * f64::from(shown)) as f32 })
    }

    fn disc(&self, node: usize, rows: &std::ops::Range<usize>) -> Option<Disc> {
        let ideal = graph::world_radius(self.graph.degrees[node], self.node_size, self.spread) * self.scale;
        let radius = ideal.clamp(graph::SUB_PIXEL_RADIUS, graph::MAX_NODE_PIXELS);
        let r = (radius * self.sf) as f32;
        let [x, y] = self.screen[node];
        if x < -r || x > self.width + r || y + r + 1.0 < rows.start as f32 || y - r - 1.0 > rows.end as f32 {
            return None;
        }
        let shown = self.reveal[node];
        if shown <= 0.0 {
            return None;
        }
        let state = self.intensity[node];
        let fill = match self.heat {
            Some((days, range)) if days[node].is_finite() && days[node] > 0.0 => {
                let span = range.newest - range.oldest;
                let t = if span > 1e-4 { ((f64::from(days[node]) - range.oldest) / span).clamp(0.0, 1.0) as f32 } else { 0.5 };
                lerp(self.cold, self.hot, t)
            }
            _ => self.node,
        };
        let dim = graph::dimmed(self.dimming, f64::from(state)) as f32;
        let amount = ((ideal / graph::SUB_PIXEL_RADIUS).clamp(0.12, 1.0) as f32).max(state) * dim;
        let rim = smoothstep(2.5, 5.0, radius) * (1.8 / radius).clamp(0.0, 0.35);
        Some(Disc { center: [x, y], radius: r, body: tone(self.back, fill, amount), rim: tone(self.back, self.rim, amount), inner: r * (1.0 - rim as f32), alpha: shown })
    }

    pub(super) fn draw(&self, band: &mut Band<'_>, stamps: &[Stamp<'_>]) {
        let rows = &self.graph.rows;
        let range = band.rows.clone();
        let (top, bottom) = (band.rows.start as f64 - 2.0, band.rows.end as f64 + 2.0);
        let (high, low) = (self.world_y(top), self.world_y(bottom));
        let margin = self.node_margin as f32;
        if self.everything {
            for index in 0..self.graph.edges.len() as u32 {
                if let Some(edge) = self.edge(index, &range) {
                    band.line(&edge);
                }
            }
            for node in 0..self.graph.len() {
                if let Some(disc) = self.disc(node, &range) {
                    band.disc(&disc);
                }
            }
        } else {
            for &index in rows.edges(low, high).iter().chain(&rows.long) {
                if let Some(edge) = self.edge(index, &range) {
                    band.line(&edge);
                }
            }
            for &node in rows.nodes(low - margin, high + margin) {
                if let Some(disc) = self.disc(node as usize, &range) {
                    band.disc(&disc);
                }
            }
        }
        for stamp in stamps {
            if stamp.y < band.rows.end as i32 && stamp.y + stamp.sprite.height as i32 > band.rows.start as i32 {
                band.stamp(stamp);
            }
        }
    }
}
