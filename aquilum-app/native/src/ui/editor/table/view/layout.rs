use masonry::core::BrushIndex;
use masonry::kurbo::{Point, Rect};
use masonry::parley::{Alignment, Layout};

use super::{ADD, GAP, Line, PAD_Y, STRIP, STRIP_PAD, TOP, Cell, TableView, letters};
use crate::ui::editor::markdown::Decor;
use crate::ui::editor::table::model::{Align, Range2, Table};
use crate::ui::scroll::bar::Span;
use crate::ui::tokens::size;

impl Table {
    pub fn layout(&self, available: f64, editing: Option<(usize, usize)>, mut make: impl FnMut(&str, Option<f64>, Alignment, bool) -> (Layout<BrushIndex>, Decor)) -> TableView {
        let (n, m) = (self.rows(), self.cols());
        let numbers: Vec<Layout<BrushIndex>> = (0..n).map(|r| make(&(r + 1).to_string(), None, Alignment::Start, false).0).collect();
        let columns: Vec<Layout<BrushIndex>> = (0..m).map(|c| make(&letters(c), None, Alignment::Start, false).0).collect();
        let corner = numbers.iter().map(|l| f64::from(l.width()) + STRIP_PAD * 2.0).fold(STRIP, f64::max);
        let scroll_w = (available - ADD).max(STRIP * 2.0);
        let widths: Vec<f64> = if self.explicit { self.widths.clone() } else { vec![((scroll_w - corner - GAP) / m as f64).max(1.0); m] };
        let mut xs = vec![0.0];
        for w in &widths {
            xs.push(xs.last().copied().unwrap_or(0.0) + w);
        }
        let layout_w = (xs[m] + corner + GAP).max(scroll_w);
        let mut heights = self.heights.clone();
        let pad = size::EDITOR_TABLE_CELL_PADDING;
        let mut placed = Vec::new();
        for r in 0..n {
            for c in 0..m {
                let region = self.region(r, c);
                if (region.top, region.left) != (r, c) {
                    continue;
                }
                let width = xs[region.right + 1] - xs[c];
                let styled = editing != Some((r, c));
                let text = self.cells[r][c].clone();
                let align = match self.aligns[c] {
                    Some(Align::Center) => Alignment::Center,
                    Some(Align::Right) => Alignment::End,
                    _ => Alignment::Start,
                };
                let (layout, decor) = make(&text, Some((width - pad * 2.0).max(1.0)), align, styled);
                let need = f64::from(layout.height()) + pad * 2.0;
                let have: f64 = heights[r..=region.bottom].iter().sum();
                if need > have {
                    heights[region.bottom] += need - have;
                }
                placed.push((r, c, region, layout, decor, text));
            }
        }
        let mut ys = vec![0.0];
        for h in &heights {
            ys.push(ys.last().copied().unwrap_or(0.0) + h);
        }
        let cells = placed
            .into_iter()
            .map(|(row, col, region, layout, decor, text)| Cell { row, col, rect: Rect::new(xs[col], ys[row], xs[region.right + 1], ys[region.bottom + 1]), layout, decor, text })
            .collect();
        let bar = if layout_w > scroll_w { size::SCROLLBAR_WIDTH } else { 0.0 };
        TableView { model: self.clone(), height: TOP + ys[n] + ADD + bar + PAD_Y, corner, scroll_w, layout_w, xs, ys, cells, columns, numbers }
    }
}

impl TableView {
    pub fn max_scroll(&self) -> f64 {
        (self.layout_w - self.scroll_w).max(0.0)
    }

    pub fn bar_track(&self) -> Option<Rect> {
        let bottom = TOP + self.ys[self.ys.len() - 1] + ADD;
        (self.max_scroll() > 0.0).then(|| Rect::new(0.0, bottom, self.scroll_w, bottom + size::SCROLLBAR_WIDTH))
    }

    pub fn bar_span(&self, scroll_x: f64) -> Option<Span> {
        Some(Span { track: self.bar_track()?, viewport: self.scroll_w, content: self.layout_w, offset: scroll_x })
    }

    pub(super) fn table_x(&self, scroll_x: f64) -> f64 {
        self.corner + GAP - scroll_x.clamp(0.0, self.max_scroll())
    }

    pub fn resized_line(&self, index: usize, width: f64, scroll_x: f64) -> Line {
        let delta = width - (self.xs[index + 1] - self.xs[index]);
        let max = (self.layout_w + delta - self.scroll_w).max(0.0);
        Line::Col(self.corner + GAP - scroll_x.clamp(0.0, max) + self.xs[index + 1] + delta, true)
    }

    pub fn cell_origin(&self, cell: &Cell, scroll_x: f64) -> Point {
        let pad = size::EDITOR_TABLE_CELL_PADDING;
        Point::new(self.table_x(scroll_x) + cell.rect.x0 + pad, TOP + cell.rect.y0 + pad)
    }

    pub fn cell(&self, r: usize, c: usize) -> Option<&Cell> {
        let (r, c) = self.model.anchor(r, c);
        self.cells.iter().find(|cell| cell.row == r && cell.col == c)
    }

    pub fn range_rect(&self, range: Range2, scroll_x: f64) -> Rect {
        let x = self.table_x(scroll_x);
        Rect::new(x + self.xs[range.left], TOP + self.ys[range.top], x + self.xs[range.right + 1], TOP + self.ys[range.bottom + 1])
    }

    pub fn widths(&self) -> Vec<f64> {
        self.xs.windows(2).map(|w| w[1] - w[0]).collect()
    }

    pub fn heights(&self) -> Vec<f64> {
        self.ys.windows(2).map(|w| w[1] - w[0]).collect()
    }
}
