use std::time::Instant;

use masonry::kurbo::Point;

use super::GraphView;
use crate::graph::{self, Graph};

impl GraphView {
    pub(super) fn shown(&self, graph: &Graph, node: usize) -> bool {
        !(f64::from(graph.created[node]) < self.display.created_from)
    }

    pub(super) fn bounds(&self) -> graph::Bounds {
        let b = self.graph.as_ref().map(|g| g.bounds).unwrap_or_default();
        let s = self.display.spread;
        graph::Bounds { min_x: b.min_x * s, min_y: b.min_y * s, max_x: b.max_x * s, max_y: b.max_y * s }
    }

    pub(super) fn glide(&mut self) {
        if self.graph.is_some() {
            self.camera.glide_to(self.bounds(), self.size.width, self.size.height);
            self.moved();
        }
    }

    pub(super) fn screen(&self, node: usize) -> Point {
        let [x, y] = self.positions[node];
        let s = self.display.spread;
        let (dx, dy) = self.camera.to_offset(f64::from(x) * s, f64::from(y) * s);
        Point::new(dx + self.size.width / 2.0, dy + self.size.height / 2.0)
    }

    pub(super) fn radius(&self, graph: &Graph, node: usize) -> f64 {
        graph::radius_pixels(graph.degrees[node], self.camera.scale, self.display.node_size, self.display.spread)
    }

    pub(super) fn node_at(&self, p: Point) -> Option<usize> {
        let graph = self.graph.as_ref()?;
        let (x, y) = self.camera.to_world(p.x - self.size.width / 2.0, p.y - self.size.height / 2.0);
        graph.pick(x, y, self.camera.scale, self.display.node_size, self.display.spread, |n| self.shown(graph, n))
    }

    pub(super) fn relight(&mut self) {
        let Some(graph) = self.graph.clone() else { return };
        for node in std::mem::take(&mut self.lit) {
            self.marks[node] = 0;
            self.moving.push(node);
        }
        self.dimming_target = if self.hovered.is_some() { 1.0 } else { 0.0 };
        if let Some(start) = self.hovered {
            let (marks, lit, moving) = (&mut self.marks, &mut self.lit, &mut self.moving);
            graph.levels(start, self.display.depth, |node, level| {
                marks[node] = level as u8 + 1;
                lit.push(node);
                moving.push(node);
            });
        }
        self.moving.sort_unstable();
        self.moving.dedup();
        self.animating = true;
    }

    pub(super) fn hover(&mut self, node: Option<usize>) -> bool {
        if node == self.hovered {
            return false;
        }
        self.hovered = node;
        self.relight();
        true
    }

    pub(super) fn moved(&mut self) {
        self.moved_at = Some(Instant::now());
        self.animating = true;
    }
}
