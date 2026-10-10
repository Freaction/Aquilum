use super::*;

const HIGHLIGHT_FADE: f64 = 0.055;
const MORPH_SECONDS: f64 = 0.45;

impl GraphView {
    pub(super) fn step(&mut self, seconds: f64) -> bool {
        let mut busy = self.camera.advance(seconds);
        if let (Some((from, progress)), Some(graph)) = (&mut self.morph, &self.graph) {
            *progress = (*progress + seconds / MORPH_SECONDS).min(1.0);
            let e = (*progress * *progress * (3.0 - 2.0 * *progress)) as f32;
            for (i, p) in self.positions.iter_mut().enumerate() {
                let (a, b) = (from[i], graph.positions[i]);
                *p = [a[0] + (b[0] - a[0]) * e, a[1] + (b[1] - a[1]) * e];
            }
            busy = true;
            if *progress >= 1.0 {
                self.morph = None;
            }
        }
        let fade = approach(seconds, HIGHLIGHT_FADE) as f32;
        let gap = self.dimming_target - self.dimming;
        self.dimming = if gap.abs() < 1.0 / 255.0 { self.dimming_target } else { self.dimming + gap * f64::from(fade) };
        busy |= self.dimming != self.dimming_target;
        let (marks, intensity) = (&self.marks, &mut self.intensity);
        self.moving.retain(|&node| {
            let goal = graph::highlight_state(marks[node]) as f32;
            let next = intensity[node] + (goal - intensity[node]) * fade;
            intensity[node] = if (next - goal).abs() < 1.0 / 31.0 { goal } else { next };
            intensity[node] != goal
        });
        busy |= !self.moving.is_empty();
        if self.revealing
            && let Some(graph) = self.graph.clone()
        {
            let step = approach(seconds, FADE) as f32;
            let mut moving = false;
            for node in 0..self.reveal.len() {
                let goal = if self.shown(&graph, node) { 1.0 } else { 0.0 };
                let current = self.reveal[node];
                if current != goal {
                    let next = current + (goal - current) * step;
                    self.reveal[node] = if (next - goal).abs() <= SETTLED as f32 { goal } else { next };
                    moving = true;
                }
            }
            self.revealing = moving;
            busy |= moving;
        }
        busy
    }

}
