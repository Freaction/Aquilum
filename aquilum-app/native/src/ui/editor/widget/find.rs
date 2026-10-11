use super::*;

#[derive(Default)]
pub(super) struct Find {
    query: String,
    matches: Vec<Range<usize>>,
    active: usize,
    only_active: bool,
}

pub(crate) fn find_all(text: &str, query: &str) -> Vec<Range<usize>> {
    let lower = |c: char| c.to_lowercase().next().unwrap_or(c);
    let query: Vec<char> = query.chars().map(lower).collect();
    if query.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let mut found = Vec::new();
    let mut i = 0;
    while i + query.len() <= chars.len() {
        if chars[i..i + query.len()].iter().zip(&query).all(|((_, c), q)| lower(*c) == *q) {
            found.push(chars[i].0..chars.get(i + query.len()).map_or(text.len(), |c| c.0));
            i += query.len();
        } else {
            i += 1;
        }
    }
    found
}

impl TextEditor {
    fn find_status(&self) -> (usize, usize) {
        let total = self.find.matches.len();
        (if total == 0 { 0 } else { self.find.active + 1 }, total)
    }

    pub(super) fn refind(&mut self) {
        if !self.hover_terms.is_empty() {
            self.find_hover_marks();
        }
        if !self.find.query.is_empty() {
            self.find.matches = find_all(self.doc.text(), &self.find.query);
            self.find.active = self.find.active.min(self.find.matches.len().saturating_sub(1));
        }
    }

    pub(super) fn block_search(&self, block: Range<usize>) -> Option<crate::ui::editor::Search> {
        if self.find.query.is_empty() {
            return None;
        }
        let query = self.find.query.clone();
        if !self.find.only_active {
            return Some(crate::ui::editor::Search { query, only: None });
        }
        let active = self.find.matches.get(self.find.active)?;
        let before = self.find.matches.iter().take_while(|m| m.start < block.start).count();
        (block.start <= active.start && active.end <= block.end).then(|| crate::ui::editor::Search { query, only: Some(self.find.active - before) })
    }

    pub(super) fn find_marks(&self) -> &[Range<usize>] {
        match self.find.matches.get(self.find.active) {
            Some(active) if self.find.only_active => std::slice::from_ref(active),
            _ => &self.find.matches,
        }
    }

    pub fn find(this: &mut WidgetMut<'_, Self>, query: &str) -> (usize, usize) {
        this.widget.find = Find { query: query.to_owned(), matches: find_all(this.widget.doc.text(), query), active: 0, only_active: false };
        this.ctx.request_render();
        this.widget.find_status()
    }

    pub fn find_step(this: &mut WidgetMut<'_, Self>, delta: isize) -> ((usize, usize), Option<usize>) {
        let total = this.widget.find.matches.len();
        if total > 0 {
            let find = &mut this.widget.find;
            find.active = (find.active as isize + delta).rem_euclid(total as isize) as usize;
            find.only_active = true;
            this.ctx.request_render();
        }
        let start = this.widget.find.matches.get(this.widget.find.active).map(|m| m.start);
        (this.widget.find_status(), start)
    }

    pub fn find_show_all(this: &mut WidgetMut<'_, Self>) {
        this.widget.find.only_active = false;
        this.ctx.request_render();
    }
}
