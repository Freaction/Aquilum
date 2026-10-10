pub mod camera;
pub mod raster;
pub mod rows;

use std::path::Path;

const HEADER: usize = 16;
pub const MIN_LAYOUT_SPACING: f64 = 0.6;
pub const MIN_SPREAD: f64 = 0.6;
const NODE_GAP_SHARE: f64 = 0.49;
const BASE_RADIUS: f64 = 0.055;
const DEGREE_RADIUS: f64 = 0.03;
const MAX_WORLD_RADIUS: f64 = 0.7;
pub const SUB_PIXEL_RADIUS: f64 = 0.75;
pub const MAX_NODE_PIXELS: f64 = 160.0;
const PICK_SLACK_PIXELS: f64 = 4.0;
const HIGHLIGHT_FALLOFF: f64 = 0.8;
const HIGHLIGHT_LIMIT: usize = 20_000;
pub const DIMMED_STRENGTH: f64 = 0.28;

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Bounds {
    pub min_x: f64,
    pub min_y: f64,
    pub max_x: f64,
    pub max_y: f64,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct Range {
    pub oldest: f64,
    pub newest: f64,
}

pub struct Grid {
    cell: f64,
    columns: usize,
    rows: usize,
    min_x: f64,
    min_y: f64,
    offsets: Vec<u32>,
    nodes: Vec<u32>,
}

pub struct Graph {
    pub positions: Vec<[f32; 2]>,
    pub created: Vec<f32>,
    pub modified: Vec<f32>,
    pub degrees: Vec<u32>,
    pub edges: Vec<[u32; 2]>,
    pub paths: Vec<String>,
    pub titles: Vec<String>,
    offsets: Vec<u32>,
    targets: Vec<u32>,
    pub grid: Option<Grid>,
    pub bounds: Bounds,
    pub created_range: Range,
    pub modified_range: Range,
    pub rows: rows::Rows,
    pub max_degree: u32,
}

fn u32_at(bytes: &[u8], at: usize) -> Option<u32> {
    Some(u32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn f32_at(bytes: &[u8], at: usize) -> Option<f32> {
    Some(f32::from_le_bytes(bytes.get(at..at + 4)?.try_into().ok()?))
}

fn range(days: &[f32]) -> Range {
    let finite = days.iter().filter(|d| d.is_finite()).map(|&d| f64::from(d));
    let (oldest, newest) = finite.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), d| (a.min(d), b.max(d)));
    if oldest.is_finite() { Range { oldest, newest } } else { Range::default() }
}

pub fn decode(bytes: &[u8]) -> Option<(Vec<[f32; 2]>, Vec<f32>, Vec<f32>, Vec<u32>, Vec<[u32; 2]>, u64)> {
    let nodes = u32_at(bytes, 0)? as usize;
    let edges = u32_at(bytes, 4)? as usize;
    let epoch = u64::from(u32_at(bytes, 8)?) | (u64::from(u32_at(bytes, 12)?) << 32);
    let mut at = HEADER;
    let mut next_f32 = |count: usize| -> Option<Vec<f32>> {
        let values = (0..count).map(|i| f32_at(bytes, at + i * 4)).collect::<Option<Vec<_>>>()?;
        at += count * 4;
        Some(values)
    };
    let flat = next_f32(nodes * 2)?;
    let created = next_f32(nodes)?;
    let modified = next_f32(nodes)?;
    let degrees = (0..nodes).map(|i| u32_at(bytes, at + i * 4)).collect::<Option<Vec<_>>>()?;
    at += nodes * 4;
    let pairs = (0..edges).map(|i| Some([u32_at(bytes, at + i * 8)?, u32_at(bytes, at + i * 8 + 4)?])).collect::<Option<Vec<_>>>()?;
    let positions = flat.chunks_exact(2).map(|p| [p[0], p[1]]).collect();
    Some((positions, created, modified, degrees, pairs, epoch))
}

pub fn load(core: &aquilum_core::Core, workspace: &Path) -> Result<Graph, String> {
    let bytes = core.search.render_graph(&workspace.to_string_lossy()).map_err(|e| format!("{e}"))?;
    let (positions, created, modified, degrees, edges, epoch) = decode(&bytes).ok_or_else(|| "снимок графа повреждён".to_owned())?;
    let indices: Vec<u32> = (0..positions.len() as u32).collect();
    let paths = core.search.graph_paths(epoch, &indices).map_err(|e| format!("{e}"))?;
    Ok(Graph::new(positions, created, modified, degrees, edges, paths))
}

impl Graph {
    pub fn new(positions: Vec<[f32; 2]>, created: Vec<f32>, modified: Vec<f32>, degrees: Vec<u32>, edges: Vec<[u32; 2]>, paths: Vec<String>) -> Graph {
        let count = positions.len();
        let mut offsets = vec![0u32; count + 1];
        for &[a, b] in &edges {
            offsets[a as usize + 1] += 1;
            offsets[b as usize + 1] += 1;
        }
        for i in 0..count {
            offsets[i + 1] += offsets[i];
        }
        let mut cursor = offsets.clone();
        let mut targets = vec![0u32; edges.len() * 2];
        for &[a, b] in &edges {
            targets[cursor[a as usize] as usize] = b;
            cursor[a as usize] += 1;
            targets[cursor[b as usize] as usize] = a;
            cursor[b as usize] += 1;
        }
        let titles = paths.iter().map(|p| Path::new(p).file_stem().map(|s| s.to_string_lossy().into_owned()).unwrap_or_default()).collect();
        let bounds = positions.iter().fold(None, |acc: Option<Bounds>, &[x, y]| {
            let (x, y) = (f64::from(x), f64::from(y));
            Some(match acc {
                None => Bounds { min_x: x, min_y: y, max_x: x, max_y: y },
                Some(b) => Bounds { min_x: b.min_x.min(x), min_y: b.min_y.min(y), max_x: b.max_x.max(x), max_y: b.max_y.max(y) },
            })
        });
        let bounds = bounds.unwrap_or_default();
        let created_range = range(&created);
        let modified_range = range(&modified);
        let rows = rows::Rows::new(&positions, &edges);
        let max_degree = degrees.iter().copied().max().unwrap_or(0);
        let mut graph = Graph { positions, created, modified, degrees, edges, paths, titles, offsets, targets, grid: None, bounds, created_range, modified_range, rows, max_degree };
        graph.grid = graph.build_grid();
        graph
    }

    pub fn len(&self) -> usize {
        self.positions.len()
    }

    pub fn neighbours(&self, node: usize) -> &[u32] {
        &self.targets[self.offsets[node] as usize..self.offsets[node + 1] as usize]
    }

    fn build_grid(&self) -> Option<Grid> {
        if self.positions.is_empty() {
            return None;
        }
        let b = self.bounds;
        let width = (b.max_x - b.min_x).max(1e-3);
        let height = (b.max_y - b.min_y).max(1e-3);
        let target = ((self.len() as f64 / 2.0).sqrt().floor()).max(1.0);
        let cell = width.max(height) / target;
        let columns = ((width / cell).ceil() as usize).max(1);
        let rows = ((height / cell).ceil() as usize).max(1);
        let cell_of: Vec<usize> = self
            .positions
            .iter()
            .map(|&[x, y]| {
                let column = (((f64::from(x) - b.min_x) / cell).floor().max(0.0) as usize).min(columns - 1);
                let row = (((f64::from(y) - b.min_y) / cell).floor().max(0.0) as usize).min(rows - 1);
                row * columns + column
            })
            .collect();
        let mut offsets = vec![0u32; columns * rows + 1];
        for &c in &cell_of {
            offsets[c + 1] += 1;
        }
        for i in 0..columns * rows {
            offsets[i + 1] += offsets[i];
        }
        let mut cursor = offsets.clone();
        let mut nodes = vec![0u32; self.len()];
        for (node, &c) in cell_of.iter().enumerate() {
            nodes[cursor[c] as usize] = node as u32;
            cursor[c] += 1;
        }
        Some(Grid { cell, columns, rows, min_x: b.min_x, min_y: b.min_y, offsets, nodes })
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

pub fn highlight_state(mark: u8) -> f64 {
    if mark < 1 { 0.0 } else { HIGHLIGHT_FALLOFF.powi(i32::from(mark) - 1) }
}

pub fn world_radius(degree: u32, size: f64, spread: f64) -> f64 {
    let ceiling = MAX_WORLD_RADIUS.min(MIN_LAYOUT_SPACING * spread * NODE_GAP_SHARE);
    ((BASE_RADIUS + DEGREE_RADIUS * f64::from(degree.max(1)).sqrt()) * size).min(ceiling)
}

pub fn radius_pixels(degree: u32, scale: f64, size: f64, spread: f64) -> f64 {
    (world_radius(degree, size, spread) * scale).clamp(SUB_PIXEL_RADIUS, MAX_NODE_PIXELS)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sample() -> Graph {
        Graph::new(
            vec![[0.0, 0.0], [1.0, 0.0], [5.0, 5.0]],
            vec![1.0, 2.0, 3.0],
            vec![4.0, 5.0, f32::NAN],
            vec![1, 2, 1],
            vec![[0, 1], [1, 2]],
            vec!["a/Первая.md".into(), "b.md".into(), "c.md".into()],
        )
    }

    #[test]
    fn decodes_the_core_snapshot() {
        let mut bytes = Vec::new();
        for v in [2u32, 1, 7, 0] {
            bytes.extend(v.to_le_bytes());
        }
        for v in [1.0f32, 2.0, 3.0, 4.0, 10.0, 11.0, 12.0, 13.0] {
            bytes.extend(v.to_le_bytes());
        }
        for v in [1u32, 1, 0, 1] {
            bytes.extend(v.to_le_bytes());
        }
        let (positions, created, modified, degrees, edges, epoch) = decode(&bytes).unwrap();
        assert_eq!(positions, [[1.0, 2.0], [3.0, 4.0]]);
        assert_eq!((created, modified, degrees, edges, epoch), (vec![10.0, 11.0], vec![12.0, 13.0], vec![1, 1], vec![[0, 1]], 7));
    }

    #[test]
    fn finds_neighbours_levels_and_nodes() {
        let graph = sample();
        assert_eq!(graph.titles[0], "Первая");
        assert_eq!(graph.neighbours(1), [0, 2]);
        let mut seen = Vec::new();
        graph.levels(0, 2, |node, level| seen.push((node, level)));
        assert_eq!(seen, [(0, 0), (1, 1), (2, 2)]);
        assert_eq!(graph.pick(1.0, 0.0, 100.0, 1.0, 1.0, |_| true), Some(1));
        assert_eq!(graph.pick(3.0, 3.0, 100.0, 1.0, 1.0, |_| true), None);
        assert_eq!(graph.modified_range, Range { oldest: 4.0, newest: 5.0 });
    }
}
