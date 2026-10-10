use masonry::kurbo::Point;
use masonry::parley::{Affinity, Cursor, Selection};

use super::model::{DEFAULT_ROW, MIN_COL, MIN_ROW, Range2, Table};
use super::view::{Hit, Line, Overlay, TableView};

const DRAG_THRESHOLD: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Drag {
    None,
    Cells { from: (usize, usize), at: Point, moved: bool },
    Strip { rows: bool, block: (usize, usize), at: Point, moved: bool, gap: Option<usize> },
    Resize { rows: bool, index: usize, at: Point, start: f64 },
    Bar { at: f64, start: f64 },
}

pub struct Session {
    pub start: usize,
    pub active: Option<(usize, usize)>,
    pub caret: usize,
    pub anchor: usize,
    pub range: Option<Range2>,
    pub range_anchor: Option<(usize, usize)>,
    pub drag: Drag,
    pub hover: Option<Hit>,
    pub scroll_x: f64,
    pub line: Line,
    pub menu_cell: Option<(usize, usize)>,
}

pub enum Outcome {
    None,
    Repaint,
    Write(Table),
    Leave,
    Remove,
}

impl Session {
    pub fn new(start: usize) -> Self {
        Session { start, active: None, caret: 0, anchor: 0, range: None, range_anchor: None, drag: Drag::None, hover: None, scroll_x: 0.0, line: Line::None, menu_cell: None }
    }

    pub fn overlay(&self, hovered: bool) -> Overlay {
        Overlay { selection: self.range, hover: self.hover, line: self.line, scroll_x: self.scroll_x, hovered }
    }

    fn activate(&mut self, view: &TableView, r: usize, c: usize, at: Option<Point>) {
        let (r, c) = view.model.anchor(r, c);
        self.active = Some((r, c));
        self.range = None;
        self.range_anchor = None;
        let value = view.model.value(r, c);
        self.caret = match (at, view.cell(r, c)) {
            (Some(p), Some(cell)) if self.active == Some((cell.row, cell.col)) => value.floor_char_boundary(Cursor::from_point(&cell.layout, p.x as f32, p.y as f32).index()),
            _ => value.len(),
        };
        self.anchor = self.caret;
    }

    fn select(&mut self, view: &TableView, range: Range2) {
        self.active = None;
        self.range = Some(view.model.expand(range));
    }

    pub fn press(&mut self, view: &TableView, p: Point, shift: bool) -> Outcome {
        let hit = view.hit(p, self.scroll_x);
        match hit {
            Hit::Cell(r, c, local) => {
                if shift {
                    let from = self.range_anchor.or(self.range.map(|g| (g.top, g.left))).unwrap_or((r, c));
                    self.range_anchor = Some(from);
                    self.select(view, Range2::span(from, (r, c)));
                    return Outcome::Repaint;
                }
                if self.active == Some((r, c)) {
                    if let Some(cell) = view.cell(r, c) {
                        self.caret = view.model.value(r, c).floor_char_boundary(Cursor::from_point(&cell.layout, local.x as f32, local.y as f32).index());
                        self.anchor = self.caret;
                    }
                } else {
                    self.activate(view, r, c, None);
                    self.drag = Drag::Cells { from: (r, c), at: p, moved: false };
                    return Outcome::Repaint;
                }
                self.drag = Drag::Cells { from: (r, c), at: p, moved: false };
                Outcome::Repaint
            }
            Hit::Column(c) => {
                let from = if shift { self.range_anchor.unwrap_or((0, c)) } else { (0, c) };
                self.range_anchor = Some(from);
                let block = view.model.expand(Range2::span(from, (view.model.rows() - 1, c)));
                self.select(view, block);
                self.drag = Drag::Strip { rows: false, block: (block.left, block.right), at: p, moved: false, gap: None };
                Outcome::Repaint
            }
            Hit::Row(r) => {
                let from = if shift { self.range_anchor.unwrap_or((r, 0)) } else { (r, 0) };
                self.range_anchor = Some(from);
                let block = view.model.expand(Range2::span(from, (r, view.model.cols() - 1)));
                self.select(view, block);
                self.drag = Drag::Strip { rows: true, block: (block.top, block.bottom), at: p, moved: false, gap: None };
                Outcome::Repaint
            }
            Hit::ColumnBorder(c) => {
                self.drag = Drag::Resize { rows: false, index: c, at: p, start: view.widths()[c] };
                Outcome::Repaint
            }
            Hit::RowBorder(r) => {
                self.drag = Drag::Resize { rows: true, index: r, at: p, start: view.heights()[r] };
                Outcome::Repaint
            }
            Hit::AddRow => {
                let mut model = view.model.clone();
                model.add_row();
                self.active = Some((model.rows() - 1, 0));
                self.caret = 0;
                self.anchor = 0;
                self.range = None;
                Outcome::Write(model)
            }
            Hit::AddCol => {
                let mut model = view.model.clone();
                model.add_col();
                self.active = Some((0, model.cols() - 1));
                self.caret = 0;
                self.anchor = 0;
                self.range = None;
                Outcome::Write(model)
            }
            Hit::Bar => {
                match view.bar_thumb(self.scroll_x) {
                    Some(thumb) if thumb.contains(p) => self.drag = Drag::Bar { at: p.x, start: self.scroll_x },
                    Some(thumb) => {
                        let page = view.scroll_w * crate::ui::scroll::bar::PAGE;
                        self.scroll_x = (self.scroll_x + if p.x < thumb.x0 { -page } else { page }).clamp(0.0, view.max_scroll());
                    }
                    None => {}
                }
                Outcome::Repaint
            }
            Hit::Empty | Hit::Outside => Outcome::Leave,
        }
    }

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
                let far = moved || (p - at).hypot() >= DRAG_THRESHOLD;
                if let Hit::Cell(r, c, local) = view.hit(p, self.scroll_x) {
                    if far && (r, c) != from {
                        self.drag = Drag::Cells { from, at, moved: true };
                        self.range_anchor = Some(from);
                        self.select(view, Range2::span(from, (r, c)));
                        return (Outcome::Repaint, Line::None);
                    }
                    if self.active == Some((r, c))
                        && let Some(cell) = view.cell(r, c)
                    {
                        self.caret = view.model.value(r, c).floor_char_boundary(Cursor::from_point(&cell.layout, local.x as f32, local.y as f32).index());
                        return (Outcome::Repaint, Line::None);
                    }
                }
                (Outcome::None, Line::None)
            }
            Drag::Strip { rows, block, at, moved, .. } => {
                let far = moved || (p - at).hypot() >= DRAG_THRESHOLD;
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
                let line = view.border_line(if rows { Hit::RowBorder(index) } else { Hit::ColumnBorder(index) }, self.scroll_x);
                (Outcome::Write(model), line)
            }
            Drag::Bar { at, start } => {
                self.scroll_x = (start + (p.x - at) * view.max_scroll() / view.bar_travel()).clamp(0.0, view.max_scroll());
                (Outcome::Repaint, Line::None)
            }
            Drag::None => {
                let hit = view.hit(p, self.scroll_x);
                let hover = Some(hit);
                let line = view.border_line(hit, self.scroll_x);
                if hover != self.hover {
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

    pub fn clear_range(&mut self, view: &TableView) -> Outcome {
        match self.range {
            Some(range) => {
                let mut model = view.model.clone();
                model.clear(range);
                Outcome::Write(model)
            }
            None => Outcome::None,
        }
    }

    fn edit(&mut self, view: &TableView, f: impl FnOnce(&mut String, &mut usize, &mut usize)) -> Outcome {
        let Some((r, c)) = self.active else { return Outcome::None };
        let mut value = view.model.value(r, c).to_owned();
        f(&mut value, &mut self.caret, &mut self.anchor);
        let mut model = view.model.clone();
        model.set(r, c, &value);
        let stored = model.value(r, c).len();
        self.caret = self.caret.min(stored);
        self.anchor = self.anchor.min(stored);
        Outcome::Write(model)
    }

    pub fn insert(&mut self, view: &TableView, text: &str) -> Outcome {
        let text = text.replace(['\r', '\n', '\t'], " ");
        self.edit(view, |value, caret, anchor| {
            let (a, b) = ((*caret).min(*anchor), (*caret).max(*anchor));
            value.replace_range(a..b, &text);
            *caret = a + text.len();
            *anchor = *caret;
        })
    }

    pub fn delete(&mut self, view: &TableView, forward: bool) -> Outcome {
        self.edit(view, |value, caret, anchor| {
            let (mut a, mut b) = ((*caret).min(*anchor), (*caret).max(*anchor));
            if a == b {
                if forward {
                    b = value[a..].chars().next().map_or(a, |ch| a + ch.len_utf8());
                } else {
                    a = value[..b].chars().next_back().map_or(b, |ch| b - ch.len_utf8());
                }
            }
            value.replace_range(a..b, "");
            *caret = a;
            *anchor = a;
        })
    }

    pub fn selected(&self, view: &TableView) -> Option<String> {
        let (r, c) = self.active?;
        let value = view.model.value(r, c);
        let (a, b) = (self.caret.min(self.anchor), self.caret.max(self.anchor));
        (a < b).then(|| value[a..b].to_owned())
    }

    pub fn step(&mut self, view: &TableView, forward: bool, word: bool, extend: bool) {
        let Some((r, c)) = self.active else { return };
        let Some(cell) = view.cell(r, c) else { return };
        let value = view.model.value(r, c);
        if !extend && self.caret != self.anchor {
            self.caret = if forward { self.caret.max(self.anchor) } else { self.caret.min(self.anchor) };
            self.anchor = self.caret;
            return;
        }
        let cursor = Cursor::from_byte_index(&cell.layout, self.caret.min(value.len()), Affinity::Downstream);
        let moved = match (forward, word) {
            (true, false) => cursor.next_visual(&cell.layout),
            (false, false) => cursor.previous_visual(&cell.layout),
            (true, true) => cursor.next_visual_word(&cell.layout),
            (false, true) => cursor.previous_visual_word(&cell.layout),
        };
        self.caret = value.floor_char_boundary(moved.index());
        if !extend {
            self.anchor = self.caret;
        }
    }

    pub fn edge(&mut self, view: &TableView, end: bool, extend: bool) {
        let Some((r, c)) = self.active else { return };
        self.caret = if end { view.model.value(r, c).len() } else { 0 };
        if !extend {
            self.anchor = self.caret;
        }
    }

    pub fn select_all(&mut self, view: &TableView) {
        if let Some((r, c)) = self.active {
            self.anchor = 0;
            self.caret = view.model.value(r, c).len();
        }
    }

    pub fn next(&mut self, view: &TableView, back: bool) -> Outcome {
        let Some((r, c)) = self.active else { return Outcome::None };
        let model = &view.model;
        let anchors: Vec<(usize, usize)> = (0..model.rows()).flat_map(|r| (0..model.cols()).map(move |c| (r, c))).filter(|(r, c)| model.anchor(*r, *c) == (*r, *c)).collect();
        let i = anchors.iter().position(|a| *a == (r, c)).unwrap_or(0);
        if back {
            let (r, c) = anchors[i.saturating_sub(1)];
            self.activate(view, r, c, None);
            return Outcome::Repaint;
        }
        match anchors.get(i + 1) {
            Some(&(r, c)) => {
                self.activate(view, r, c, None);
                Outcome::Repaint
            }
            None => {
                let mut model = model.clone();
                model.add_row();
                self.active = Some((model.rows() - 1, 0));
                self.caret = 0;
                self.anchor = 0;
                Outcome::Write(model)
            }
        }
    }

    pub fn down(&mut self, view: &TableView) -> Outcome {
        let Some((r, c)) = self.active else { return Outcome::None };
        let below = view.model.region(r, c).bottom + 1;
        if below < view.model.rows() {
            self.activate(view, below, c, None);
            return Outcome::Repaint;
        }
        let mut model = view.model.clone();
        model.add_row();
        if let Some(h) = model.heights.last_mut() {
            *h = DEFAULT_ROW;
        }
        self.active = Some((model.rows() - 1, c));
        self.caret = 0;
        self.anchor = 0;
        Outcome::Write(model)
    }

    pub fn caret_geometry(&self, view: &TableView) -> Option<(Point, f64, Vec<masonry::kurbo::Rect>)> {
        let (r, c) = self.active?;
        let cell = view.cell(r, c)?;
        let origin = view.cell_origin(cell, self.scroll_x);
        let value = view.model.value(r, c);
        let cursor = Cursor::from_byte_index(&cell.layout, self.caret.min(value.len()), Affinity::Downstream);
        let g = cursor.geometry(&cell.layout, 1.0);
        let selection = Selection::new(Cursor::from_byte_index(&cell.layout, self.anchor.min(value.len()), Affinity::Downstream), cursor);
        let rects = selection.geometry(&cell.layout).into_iter().map(|(b, _)| masonry::kurbo::Rect::new(b.x0, b.y0, b.x1, b.y1) + origin.to_vec2()).collect();
        Some((Point::new(origin.x + g.x0, origin.y + g.y0), g.y1 - g.y0, rects))
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
