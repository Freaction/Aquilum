mod frame;

use super::labels::readable;
use super::*;
use frame::{Frame, channels, rgb, tone};

const EDGE_WORLD_HALF_WIDTH: f64 = 0.004;
const EDGE_MIN_HALF: f64 = 0.35;
const EDGE_MAX_HALF: f64 = 0.9;
const EDGE_REFERENCE: f64 = 20_000.0;

impl GraphView {
    pub(super) fn scene(&mut self) -> Option<(ImageBrush, f64)> {
        let _zone = aq_trace::zone("граф: сцена");
        let graph = self.graph.clone()?;
        let tm = theme::current();
        let sf = self.scale_factor;
        let (width, height) = ((self.size.width * sf).round() as usize, (self.size.height * sf).round() as usize);
        if width == 0 || height == 0 {
            return None;
        }
        let s = self.display.spread;
        let (camera, cy, scale) = (self.camera, self.camera.center_y, self.camera.scale);
        let (half_w, half_h) = (self.size.width / 2.0, self.size.height / 2.0);
        {
            use rayon::prelude::*;
            let _zone = aq_trace::zone("граф: экран");
            self.screen.resize(graph.len(), [0.0; 2]);
            self.screen.par_iter_mut().zip(self.positions.par_iter()).for_each(|(out, &[x, y])| {
                let (dx, dy) = camera.to_offset(f64::from(x) * s, f64::from(y) * s);
                *out = [((dx + half_w) * sf) as f32, ((dy + half_h) * sf) as f32];
            });
        }
        let ideal = EDGE_WORLD_HALF_WIDTH * scale;
        let half = ideal.clamp(EDGE_MIN_HALF, EDGE_MAX_HALF);
        let density = (EDGE_REFERENCE / (graph.edges.len().max(1) as f64)).sqrt().clamp(0.3, 1.0);
        let thickness = (half * 2.0 * sf).min(1.0);
        let biggest = graph::radius_pixels(graph.max_degree, scale, self.display.node_size, s) + 2.0;
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
        for &(node, fade, focused) in &self.shown_labels {
            let Some(sprite) = self.sprites.get(&node) else { continue };
            let alpha = fade * if focused { 1.0 } else { read } * graph::dimmed(self.dimming, f64::from(self.intensity[node]));
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
        let image = surfaces.frame(width as u16, height as u16, |pixels| raster::bands(width, height, pixels, rgb(tm.graph_backdrop), |band| frame.draw(band, &stamps)));
        drop(stamps);
        self.surfaces = surfaces;
        Some((image, sf))
    }
}
