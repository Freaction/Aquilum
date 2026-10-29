use masonry::parley::{Affinity, Cursor};

use super::{Outcome, Session};
use crate::ui::editor::caret;
use crate::ui::editor::table::view::TableView;

impl Session {
    pub fn step(&mut self, view: &TableView, forward: bool, word: bool, extend: bool) {
        let Some((r, c)) = self.active else { return };
        let Some(cell) = view.cell(r, c) else { return };
        let value = view.model.value(r, c);
        if !extend && !word && self.caret != self.anchor {
            self.caret = if forward { self.caret.max(self.anchor) } else { self.caret.min(self.anchor) };
            self.anchor = self.caret;
            return;
        }
        let from = Cursor::from_byte_index(&cell.layout, self.caret.min(value.len()), Affinity::Downstream);
        let hop = |cursor| caret::step(&cell.layout, value, cursor, forward, word);
        let mut moved = hop(from);
        if moved.index() == from.index() {
            moved = hop(moved);
        }
        self.caret = moved.index();
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
            None => self.append_row(view, 0),
        }
    }

    pub fn down(&mut self, view: &TableView) -> Outcome {
        let Some((r, c)) = self.active else { return Outcome::None };
        let below = view.model.region(r, c).bottom + 1;
        if below < view.model.rows() {
            self.activate(view, below, c, None);
            return Outcome::Repaint;
        }
        self.append_row(view, c)
    }
}
