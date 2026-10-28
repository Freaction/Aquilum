mod build;
pub mod camera;
mod metrics;
mod query;
pub mod raster;
pub mod rows;
mod snapshot;
#[cfg(test)]
mod tests;

pub use metrics::{MAX_NODE_PIXELS, MIN_SPREAD, SUB_PIXEL_RADIUS, dimmed, highlight_state, radius_pixels, world_radius};
pub use snapshot::load;

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
