mod book;
mod dataview;
mod drawing;
mod math;
pub mod media;
mod metrics;
pub mod panel;
mod spec;

use masonry::imaging::Painter;
use masonry::peniko::Brush;

use super::table::{Overlay, TableView};
use drawing::{Item, Tone, card, link_hits};

pub use book::{Row as BookRow, books};
pub use dataview::dataview;
pub use drawing::{Action, Drawing};
pub use math::{Formula, formula, math};
pub use media::{MediaHit, MediaOverlay, MediaView};
pub use spec::{Font, Spec};

pub enum View {
    Table(TableView),
    Media(MediaView),
    Drawing(Drawing),
}

impl View {
    pub fn height(&self) -> f64 {
        match self {
            View::Table(t) => t.height,
            View::Media(m) => m.height,
            View::Drawing(d) => d.height,
        }
    }

    pub fn decors(&self) -> impl Iterator<Item = &super::markdown::Decor> {
        let cells = match self {
            View::Table(t) => t.cells.as_slice(),
            _ => &[],
        };
        cells.iter().map(|cell| &cell.decor)
    }

    pub fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, palette: &[Brush], overlay: &Overlay, media: &MediaOverlay, hover: Option<usize>) {
        match self {
            View::Table(t) => t.paint(painter, x, y, palette, overlay),
            View::Media(m) => m.paint(painter, x, y, media),
            View::Drawing(d) => d.paint(painter, x, y, palette, hover, overlay.search.as_ref()),
        }
    }
}
