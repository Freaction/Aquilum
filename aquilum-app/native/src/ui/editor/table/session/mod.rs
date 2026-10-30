use std::ops::Range;

use masonry::kurbo::{Axis, Point};

use super::model::{Range2, Table};
use super::view::{Hit, Line, Overlay, TableView};
use crate::ui::editor::caret;
use crate::ui::scroll::bar::Scrollbar;

mod drag;
mod edit;
mod nav;
mod press;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    None,
    Cells { from: (usize, usize), at: Point, moved: bool },
    Strip { rows: bool, block: (usize, usize), at: Point, moved: bool, gap: Option<usize> },
    Resize { rows: bool, index: usize, at: Point, start: f64 },
    Bar,
}

pub struct Session {
    pub start: usize,
    pub active: Option<(usize, usize)>,
    pub caret: usize,
    pub anchor: usize,
    pub compose: Option<Range<usize>>,
    pub range: Option<Range2>,
    pub range_anchor: Option<(usize, usize)>,
    pub drag: Drag,
    pub hover: Option<Hit>,
    pub scroll_x: f64,
    pub line: Line,
    pub menu_cell: Option<(usize, usize)>,
    pub bar: Scrollbar,
}

pub enum Outcome {
    None,
    Repaint,
    Write(Table),
    Leave,
    Remove,
    Copy(String),
}

impl Session {
    pub fn new(start: usize) -> Self {
        Session { start, active: None, caret: 0, anchor: 0, compose: None, range: None, range_anchor: None, drag: Drag::None, hover: None, scroll_x: 0.0, line: Line::None, menu_cell: None, bar: Scrollbar::new(Axis::Horizontal) }
    }

    pub fn overlay(&self, hovered: bool) -> Overlay {
        Overlay { selection: self.range, hover: self.hover, line: self.line, scroll_x: self.scroll_x, hovered: hovered || self.active.is_some(), bar_active: self.bar.active(), ..Overlay::default() }
    }

    pub(super) fn enter(&mut self, cell: (usize, usize), caret: usize) {
        self.active = Some(cell);
        self.range = None;
        self.range_anchor = None;
        self.caret = caret;
        self.anchor = caret;
        self.compose = None;
    }

    fn activate(&mut self, view: &TableView, r: usize, c: usize, at: Option<Point>) {
        let (r, c) = view.model.anchor(r, c);
        let value = view.model.value(r, c);
        let caret = match (at, view.cell(r, c)) {
            (Some(p), Some(cell)) => caret::hit(&cell.layout, value, p).0,
            _ => value.len(),
        };
        self.enter((r, c), caret);
    }

    fn select(&mut self, view: &TableView, range: Range2) {
        self.active = None;
        self.range = Some(view.model.expand(range));
    }

    fn append_row(&mut self, view: &TableView, col: usize) -> Outcome {
        let mut model = view.model.clone();
        model.add_row();
        self.enter((model.rows() - 1, col), 0);
        Outcome::Write(model)
    }
}
