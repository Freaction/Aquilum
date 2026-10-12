mod geometry;
mod layout;
mod paginate;
mod text;

use masonry::core::BrushIndex;
use masonry::kurbo::Size;
use masonry::parley::Layout;
use masonry::peniko::ImageBrush;

pub use geometry::{Geometry, advance, geometry};
pub use layout::layout_blocks;
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

pub enum Item {
    Text { layout: Layout<BrushIndex>, x: f64, before: f64, after: f64, breaks: Vec<(usize, char)> },
    Image { image: Option<ImageBrush>, size: Size, before: f64, after: f64 },
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
