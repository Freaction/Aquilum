use masonry::kurbo::{Point, Rect};
use masonry::parley::{Affinity, Cursor};

use super::{Outcome, Session};
use crate::ui::editor::markdown::range_rects;
use crate::ui::editor::table::view::TableView;

impl Session {
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

    pub fn delete(&mut self, view: &TableView, forward: bool, word: bool) -> Outcome {
        if self.caret == self.anchor {
            self.step(view, forward, word, true);
        }
        self.insert(view, "")
    }

    pub fn delete_around(&mut self, view: &TableView, before: usize, after: usize) -> Outcome {
        self.edit(view, |value, caret, anchor| {
            let (a, b) = ((*caret).min(*anchor), (*caret).max(*anchor));
            let start = value.floor_char_boundary(a.saturating_sub(before));
            let end = value.floor_char_boundary((b + after).min(value.len()));
            value.replace_range(b..end, "");
            value.replace_range(start..a, "");
            (*caret, *anchor) = (*caret - (a - start), *anchor - (a - start));
        })
    }

    pub fn range_text(&self, view: &TableView) -> Option<String> {
        let range = self.range?;
        let row = |r: usize| (range.left..=range.right).map(|c| view.model.value(r, c).to_owned()).collect::<Vec<_>>().join("\t");
        Some((range.top..=range.bottom).map(row).collect::<Vec<_>>().join("\n"))
    }

    pub fn selected(&self, view: &TableView) -> Option<String> {
        let (r, c) = self.active?;
        let value = view.model.value(r, c);
        let (a, b) = (self.caret.min(self.anchor), self.caret.max(self.anchor));
        (a < b).then(|| value[a..b].to_owned())
    }

    pub fn caret_geometry(&self, view: &TableView) -> Option<(Point, f64, Vec<Rect>)> {
        let (r, c) = self.active?;
        let cell = view.cell(r, c)?;
        let origin = view.cell_origin(cell, self.scroll_x);
        let len = view.model.value(r, c).len();
        let g = Cursor::from_byte_index(&cell.layout, self.caret.min(len), Affinity::Downstream).geometry(&cell.layout, 1.0);
        let rects = range_rects(&cell.layout, self.caret.min(self.anchor).min(len), self.caret.max(self.anchor).min(len)).into_iter().map(|(rect, _)| rect + origin.to_vec2()).collect();
        Some((Point::new(origin.x + g.x0, origin.y + g.y0), g.y1 - g.y0, rects))
    }
}
