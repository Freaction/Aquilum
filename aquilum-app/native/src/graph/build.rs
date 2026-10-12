use std::path::Path;

use super::{Bounds, Graph, Grid, Range, rows};

fn range(days: &[f32]) -> Range {
    let finite = days.iter().filter(|d| d.is_finite()).map(|&d| f64::from(d));
    let (oldest, newest) = finite.fold((f64::INFINITY, f64::NEG_INFINITY), |(a, b), d| (a.min(d), b.max(d)));
    if oldest.is_finite() { Range { oldest, newest } } else { Range::default() }
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
}
