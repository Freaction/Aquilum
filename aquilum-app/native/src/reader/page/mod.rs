mod geometry;
mod layout;
mod paginate;
mod text;
#[cfg(test)]
mod tests;

use masonry::kurbo::Size;

pub use geometry::{Geometry, advance, geometry};
pub use layout::{dash_width, layout_blocks, text_layout};
pub use paginate::paginate;

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub family: String,
    pub size: f32,
    pub weight: f32,
    pub line_height: f32,
    pub justify: bool,
    pub hyphenate: bool,
    pub max_width_ch: u32,
    pub margin: f64,
    pub scrolled: bool,
}

#[derive(Clone, Copy, Debug)]
pub struct Line {
    pub top: f64,
    pub bottom: f64,
    pub start: usize,
    pub end: usize,
}

pub enum Item {
    Text { lines: Vec<Line>, x: f64, before: f64, after: f64, breaks: Vec<(usize, char)> },
    Image { size: Size, before: f64, after: f64 },
    Rule,
}

impl Item {
    fn margins(&self) -> (f64, f64) {
        match self {
            Item::Text { before, after, .. } | Item::Image { before, after, .. } => (*before, *after),
            Item::Rule => (8.0, 8.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub item: usize,
    pub from: f64,
    pub to: f64,
    pub y: f64,
}

#[derive(Default, Debug)]
pub struct Column {
    pub pieces: Vec<Piece>,
}

pub struct Laid {
    pub items: Vec<Item>,
    pub columns: Vec<Column>,
    pub height: f64,
}
