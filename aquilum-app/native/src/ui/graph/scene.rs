use super::*;
use super::labels::readable;
use crate::graph::raster::Band;

const EDGE_WORLD_HALF_WIDTH: f64 = 0.004;
const EDGE_MIN_HALF: f64 = 0.35;
const EDGE_MAX_HALF: f64 = 0.9;
const EDGE_REFERENCE: f64 = 20_000.0;

type Channels = [f32; 4];

fn channels(c: Color) -> Channels {
    c.components
}

fn rgb(c: Color) -> Rgb {
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

fn tone(back: Channels, c: Channels, amount: f32) -> Rgb {
    let a = (c[3] * amount).clamp(0.0, 1.0);
    let ch = |i: usize| ((back[i] + (c[i] - back[i]) * a).clamp(0.0, 1.0) * 255.0 + 0.5) as u8;
    [ch(0), ch(1), ch(2)]
}

struct Frame<'a> {
    graph: &'a Graph,
    screen: &'a [[f32; 2]],
    reveal: &'a [f32],
    marks: &'a [u8],
    intensity: &'a [f32],
    width: f32,
    height: f32,
    sf: f64,
    scale: f64,
    spread: f64,
    center_y: f64,
    half_h: f64,
    node_size: f64,
    dimming: f64,
    edge_alpha: f64,
    edge_active_alpha: f64,
    coverage: f64,
    back: Channels,
    edge: Channels,
    edge_active: Channels,
    node: Channels,
    rim: Channels,
    cold: Channels,
    hot: Channels,
    heat: Option<(&'a [f32], graph::Range)>,
    node_margin: f64,
    everything: bool,
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
        let dim = 1.0 + self.dimming * ((graph::DIMMED_STRENGTH + (1.0 - graph::DIMMED_STRENGTH) * active) - 1.0);
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
        let dim = 1.0 + self.dimming as f32 * ((graph::DIMMED_STRENGTH as f32 + (1.0 - graph::DIMMED_STRENGTH as f32) * state) - 1.0);
        let amount = ((ideal / graph::SUB_PIXEL_RADIUS).clamp(0.12, 1.0) as f32).max(state) * dim;
        let rim = smoothstep(2.5, 5.0, radius) * (1.8 / radius).clamp(0.0, 0.35);
        Some(Disc { center: [x, y], radius: r, body: tone(self.back, fill, amount), rim: tone(self.back, self.rim, amount), inner: r * (1.0 - rim as f32), alpha: shown })
    }

    fn draw(&self, band: &mut Band<'_>, stamps: &[Stamp<'_>]) {
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

impl GraphView {
    pub(super) fn scene(&mut self) -> Option<(ImageBrush, f64)> {
        let _zone = aq_trace::zone("граф: сцена");
        let graph = self.graph.clone()?;
        let tm = theme::current();
        let sf = if self.draft { self.scale_factor * DRAFT_SCALE } else { self.scale_factor };
        let (width, height) = ((self.size.width * sf).round() as usize, (self.size.height * sf).round() as usize);
        if width == 0 || height == 0 {
            return None;
        }
        let s = self.display.spread;
        let (cx, cy, scale) = (self.camera.center_x, self.camera.center_y, self.camera.scale);
        let (half_w, half_h) = (self.size.width / 2.0, self.size.height / 2.0);
        {
            use rayon::prelude::*;
            let _zone = aq_trace::zone("граф: экран");
            self.screen.resize(graph.len(), [0.0; 2]);
            self.screen.par_iter_mut().zip(self.positions.par_iter()).for_each(|(out, &[x, y])| {
                *out = [(((f64::from(x) * s - cx) * scale + half_w) * sf) as f32, ((half_h - (f64::from(y) * s - cy) * scale) * sf) as f32];
            });
        }
        let ideal = EDGE_WORLD_HALF_WIDTH * scale;
        let half = ideal.clamp(EDGE_MIN_HALF, EDGE_MAX_HALF);
        let density = (EDGE_REFERENCE / (graph.edges.len().max(1) as f64)).sqrt().clamp(0.3, 1.0);
        let thickness = (half * 2.0 * sf).min(1.0);
        let biggest = (graph::world_radius(graph.max_degree, self.display.node_size, s) * scale).clamp(graph::SUB_PIXEL_RADIUS, graph::MAX_NODE_PIXELS) + 2.0;
        let frame = Frame {
            graph: &graph,
            screen: &self.screen,
            reveal: &self.reveal,
            marks: &self.marks,
            intensity: &self.intensity,
            width: width as f32,
            height: height as f32,
            sf,
            scale,
            spread: s,
            center_y: cy,
            half_h,
            node_size: self.display.node_size,
            dimming: self.dimming,
            edge_alpha: f64::from(tm.graph_edge.components[3]) * density * thickness,
            edge_active_alpha: density * thickness,
            coverage: (ideal / EDGE_MIN_HALF).clamp(0.15, 1.0),
            back: channels(tm.graph_backdrop),
            edge: channels(tm.graph_edge),
            edge_active: channels(tm.graph_edge_active),
            node: channels(tm.graph_node),
            rim: channels(tm.graph_node_rim),
            cold: channels(tm.graph_heat_cold),
            hot: channels(tm.graph_heat_hot),
            heat: match self.display.heat {
                Heat::None => None,
                Heat::Modified => Some((&graph.modified, graph.modified_range)),
                Heat::Created => Some((&graph.created, graph.created_range)),
            },
            node_margin: biggest / (scale * s),
            everything: self.morph.is_some(),
        };
        let read = readable(scale * s);
        let mut stamps = Vec::new();
        for &(node, fade, focused) in self.shown_labels.iter().filter(|_| !self.draft) {
            let Some(sprite) = self.sprites.get(&node) else { continue };
            let alpha = fade * if focused { 1.0 } else { read } * (1.0 + self.dimming * ((graph::DIMMED_STRENGTH + (1.0 - graph::DIMMED_STRENGTH) * f64::from(self.intensity[node])) - 1.0));
            if alpha <= SETTLED {
                continue;
            }
            let [x, y] = self.screen[node];
            let top = f64::from(y) + (self.radius(&graph, node) + LABEL_GAP) * sf;
            let color = if focused { tm.graph_label_focus } else { tm.graph_label };
            stamps.push(Stamp {
                sprite,
                x: (f64::from(x) - sprite.width as f64 / 2.0).round() as i32,
                y: (top - 1.0).round() as i32,
                fill: tone(frame.back, channels(color), 1.0),
                halo: rgb(tm.graph_label_halo),
                alpha: alpha as f32,
            });
        }
        let _zone = aq_trace::zone("граф: растр");
        let mut surfaces = std::mem::take(&mut self.surfaces);
        let mut image = surfaces.frame(width as u16, height as u16, rgb(tm.graph_backdrop), |pixels| raster::bands(width, height, pixels, |band| frame.draw(band, &stamps)));
        drop(stamps);
        self.surfaces = surfaces;
        if self.draft {
            image = image.with_quality(masonry::peniko::ImageQuality::Medium);
        }
        Some((image, sf))
    }
}
