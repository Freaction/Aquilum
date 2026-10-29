use masonry::kurbo::Point;

use super::{Drag, Outcome, Session};
use crate::ui::editor::caret;
use crate::ui::editor::table::model::Range2;
use crate::ui::editor::table::view::{Hit, TableView};

impl Session {
    pub fn press(&mut self, view: &TableView, p: Point, shift: bool, count: u8, grip: f64) -> Outcome {
        match view.hit(p, self.scroll_x, grip) {
            Hit::Cell(r, c, local) => self.press_cell(view, p, (r, c), local, shift, count),
            Hit::Column(c) => self.press_strip(view, p, shift, false, c),
            Hit::Row(r) => self.press_strip(view, p, shift, true, r),
            Hit::ColumnBorder(c) => {
                self.drag = Drag::Resize { rows: false, index: c, at: p, start: view.widths()[c] };
                Outcome::Repaint
            }
            Hit::RowBorder(r) => {
                self.drag = Drag::Resize { rows: true, index: r, at: p, start: view.heights()[r] };
                Outcome::Repaint
            }
            Hit::AddRow => self.append_row(view, 0),
            Hit::AddCol => {
                let mut model = view.model.clone();
                model.add_col();
                self.enter((0, model.cols() - 1), 0);
                Outcome::Write(model)
            }
            Hit::Bar => {
                if let Some(next) = view.bar_span(self.scroll_x).and_then(|span| self.bar.press(p, &span)) {
                    self.scroll_x = next;
                    if self.bar.dragging() {
                        self.drag = Drag::Bar;
                    }
                }
                Outcome::Repaint
            }
            Hit::Empty | Hit::Outside => Outcome::Leave,
        }
    }

    fn press_cell(&mut self, view: &TableView, p: Point, (r, c): (usize, usize), local: Point, shift: bool, count: u8) -> Outcome {
        if shift {
            let from = self.range_anchor.or(self.range.map(|g| (g.top, g.left))).unwrap_or((r, c));
            self.range_anchor = Some(from);
            self.select(view, Range2::span(from, (r, c)));
            return Outcome::Repaint;
        }
        if self.active != Some((r, c)) {
            self.activate(view, r, c, Some(local));
        } else if let Some(cell) = view.cell(r, c) {
            let unit = caret::unit(&cell.layout, view.model.value(r, c), local, count);
            (self.anchor, self.caret) = (unit.start, unit.end);
        }
        self.drag = Drag::Cells { from: (r, c), at: p, moved: false };
        Outcome::Repaint
    }

    fn press_strip(&mut self, view: &TableView, p: Point, shift: bool, rows: bool, index: usize) -> Outcome {
        let model = &view.model;
        let (first, last) = if rows { ((index, 0), (index, model.cols() - 1)) } else { ((0, index), (model.rows() - 1, index)) };
        let from = if shift { self.range_anchor.unwrap_or(first) } else { first };
        self.range_anchor = Some(from);
        let block = model.expand(Range2::span(from, last));
        self.select(view, block);
        let span = if rows { (block.top, block.bottom) } else { (block.left, block.right) };
        self.drag = Drag::Strip { rows, block: span, at: p, moved: false, gap: None };
        Outcome::Repaint
    }
}
