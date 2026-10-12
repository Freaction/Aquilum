use super::{Graph, MAX_NODE_PIXELS, radius_pixels};

const PICK_SLACK_PIXELS: f64 = 4.0;
const HIGHLIGHT_LIMIT: usize = 20_000;

impl Graph {
    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn neighbours(&self, node: usize) -> &[u32] {
        &self.targets[self.offsets[node] as usize..self.offsets[node + 1] as usize]
    }

    pub fn visit_area(&self, min_x: f64, min_y: f64, max_x: f64, max_y: f64, mut visit: impl FnMut(usize)) {
        let Some(g) = &self.grid else { return };
        let column = |x: f64| ((x - g.min_x) / g.cell).floor();
        let row = |y: f64| ((y - g.min_y) / g.cell).floor();
        let (c0, c1) = (column(min_x).max(0.0) as usize, column(max_x).min(g.columns as f64 - 1.0));
        let (r0, r1) = (row(min_y).max(0.0) as usize, row(max_y).min(g.rows as f64 - 1.0));
        if c1 < 0.0 || r1 < 0.0 {
            return;
        }
        for r in r0..=r1 as usize {
            for c in c0..=c1 as usize {
                let cell = r * g.columns + c;
                for &node in &g.nodes[g.offsets[cell] as usize..g.offsets[cell + 1] as usize] {
                    visit(node as usize);
                }
            }
        }
    }

    pub fn pick(&self, world_x: f64, world_y: f64, scale: f64, size: f64, spread: f64, shown: impl Fn(usize) -> bool) -> Option<usize> {
        let per_unit = scale * spread;
        let (tx, ty) = (world_x / spread, world_y / spread);
        let reach = (MAX_NODE_PIXELS + PICK_SLACK_PIXELS) / per_unit;
        let mut best = None;
        let mut best_distance = f64::INFINITY;
        self.visit_area(tx - reach, ty - reach, tx + reach, ty + reach, |node| {
            let [x, y] = self.positions[node];
            let distance = (f64::from(x) - tx).hypot(f64::from(y) - ty);
            let radius = (radius_pixels(self.degrees[node], scale, size, spread) + PICK_SLACK_PIXELS) / per_unit;
            if distance <= radius && distance < best_distance && shown(node) {
                best_distance = distance;
                best = Some(node);
            }
        });
        best
    }

    pub fn levels(&self, start: usize, depth: usize, mut visit: impl FnMut(usize, usize)) {
        let mut seen = std::collections::HashSet::from([start]);
        let mut frontier = vec![start];
        visit(start, 0);
        for level in 1..=depth {
            if frontier.is_empty() || seen.len() >= HIGHLIGHT_LIMIT {
                break;
            }
            let mut next = Vec::new();
            'outer: for &node in &frontier {
                for &n in self.neighbours(node) {
                    let n = n as usize;
                    if seen.insert(n) {
                        next.push(n);
                        visit(n, level);
                        if seen.len() >= HIGHLIGHT_LIMIT {
                            break 'outer;
                        }
                    }
                }
            }
            frontier = next;
        }
    }
}
