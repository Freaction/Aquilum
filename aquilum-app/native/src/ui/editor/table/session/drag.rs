use masonry::kurbo::Point;

use super::{Drag, Outcome, Session, caret_in};
use crate::ui::editor::table::model::{MIN_COL, MIN_ROW, Range2, Table};
use crate::ui::editor::table::view::{Hit, Line, TableView};
use crate::ui::keymap::gesture::DRAG_SLOP;

impl Session {
    pub fn moved(&mut self, view: &TableView, p: Point) -> Outcome {
        let (outcome, line) = self.track(view, p);
        if line != self.line {
            self.line = line;
            return match outcome {
                Outcome::None => Outcome::Repaint,
                other => other,
            };
        }
        outcome
    }

    fn track(&mut self, view: &TableView, p: Point) -> (Outcome, Line) {
        match self.drag {
            Drag::Cells { from, at, moved } => {
                let far = moved || (p - at).hypot() >= DRAG_SLOP;
                if let Hit::Cell(r, c, local) = view.hit(p, self.scroll_x, self.grip) {
                    if far && (r, c) != from {
                        self.drag = Drag::Cells { from, at, moved: true };
                        self.range_anchor = Some(from);
                        self.select(view, Range2::span(from, (r, c)));
                        return (Outcome::Repaint, Line::None);
                    }
                    if self.active == Some((r, c))
                        && let Some(cell) = view.cell(r, c)
                    {
                        self.caret = caret_in(cell, view.model.value(r, c), local);
                        return (Outcome::Repaint, Line::None);
                    }
                }
                (Outcome::None, Line::None)
            }
            Drag::Strip { rows, block, at, moved, .. } => {
                let far = moved || (p - at).hypot() >= DRAG_SLOP;
                if !far {
                    return (Outcome::None, Line::None);
                }
                let along = if rows { p.y } else { p.x };
                let count = if rows { view.model.rows() } else { view.model.cols() };
                let raw = view.gap(along, rows, self.scroll_x);
                let gap = valid_gap(&view.model, rows, block, raw, count);
                self.drag = Drag::Strip { rows, block, at, moved: true, gap };
                let line = match gap {
                    Some(g) => view.gap_line(g, rows, self.scroll_x, true),
                    None => view.gap_line(raw, rows, self.scroll_x, false),
                };
                (Outcome::Repaint, line)
            }
            Drag::Resize { rows, index, at, start } => {
                let mut model = view.model.clone();
                if rows {
                    model.heights[index] = (start + p.y - at.y).max(MIN_ROW).round();
                } else {
                    if !model.explicit {
                        model.widths = view.widths().iter().map(|w| w.round()).collect();
                        model.explicit = true;
                    }
                    model.widths[index] = (start + p.x - at.x).max(MIN_COL).round();
                }
                let line = if rows { view.border_line(Hit::RowBorder(index), self.scroll_x) } else { view.resized_line(index, model.widths[index], self.scroll_x) };
                (Outcome::Write(model), line)
            }
            Drag::Bar => {
                if let Some(next) = view.bar_span(self.scroll_x).and_then(|span| self.bar.drag(p, &span)) {
                    self.scroll_x = next;
                }
                (Outcome::Repaint, Line::None)
            }
            Drag::None => {
                let hit = view.hit(p, self.scroll_x, self.grip);
                let hover = Some(hit);
                let line = view.border_line(hit, self.scroll_x);
                let thumb = view.bar_span(self.scroll_x).is_some_and(|span| self.bar.hover(Some(p), &span));
                if hover != self.hover || thumb {
                    self.hover = hover;
                    return (Outcome::Repaint, line);
                }
                (Outcome::None, line)
            }
        }
    }

    pub fn release(&mut self, view: &TableView) -> Outcome {
        let drag = std::mem::replace(&mut self.drag, Drag::None);
        self.line = Line::None;
        self.bar.release();
        if let Drag::Strip { rows, block, moved: true, gap: Some(gap), .. } = drag {
            let mut model = view.model.clone();
            let size = block.1 - block.0;
            let dest = if gap <= block.0 { gap } else { gap - size - 1 };
            let done = if rows { model.move_rows(block.0, block.1, dest) } else { model.move_cols(block.0, block.1, dest) };
            if done {
                self.range = Some(if rows { Range2 { top: dest, left: 0, bottom: dest + size, right: model.cols() - 1 } } else { Range2 { top: 0, left: dest, bottom: model.rows() - 1, right: dest + size } });
                return Outcome::Write(model);
            }
        }
        Outcome::Repaint
    }
}

fn valid_gap(model: &Table, rows: bool, block: (usize, usize), raw: usize, count: usize) -> Option<usize> {
    let ok = |g: usize| {
        if g == block.0 || g == block.1 + 1 {
            return false;
        }
        if g > block.0 && g <= block.1 {
            return false;
        }
        !model.merges.iter().any(|m| if rows { m.top < g && g <= m.bottom } else { m.left < g && g <= m.right })
    };
    if ok(raw) {
        return Some(raw);
    }
    (1..=count).find_map(|d| [raw.checked_sub(d), Some(raw + d).filter(|g| *g <= count)].into_iter().flatten().find(|g| ok(*g)))
}
