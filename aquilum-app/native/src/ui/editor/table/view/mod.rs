use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::Layout;

use super::model::{Range2, Table};
use crate::ui::editor::markdown::Decor;

mod hit;
mod layout;
mod paint;

const PAD_Y: f64 = 12.0;
const STRIP: f64 = 24.0;
const STRIP_PAD: f64 = 4.0;
const ADD: f64 = 20.0;
const GAP: f64 = 2.0;
const PLUS: f64 = 16.0;
pub const TOP: f64 = PAD_Y + STRIP + GAP;

pub fn letters(mut n: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub rect: Rect,
    pub layout: Layout<BrushIndex>,
    pub decor: Decor,
    pub text: String,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Cell(usize, usize, Point),
    Column(usize),
    Row(usize),
    ColumnBorder(usize),
    RowBorder(usize),
    AddRow,
    AddCol,
    Bar,
    Empty,
    Outside,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Line {
    #[default]
    None,
    Row(f64, bool),
    Col(f64, bool),
}

#[derive(Default)]
pub struct Overlay {
    pub selection: Option<Range2>,
    pub hover: Option<Hit>,
    pub line: Line,
    pub scroll_x: f64,
    pub hovered: bool,
    pub bar_active: bool,
    pub search: Option<crate::ui::editor::Search>,
}

pub struct TableView {
    pub model: Table,
    pub height: f64,
    pub corner: f64,
    pub scroll_w: f64,
    pub layout_w: f64,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub cells: Vec<Cell>,
    columns: Vec<Layout<BrushIndex>>,
    numbers: Vec<Layout<BrushIndex>>,
}
