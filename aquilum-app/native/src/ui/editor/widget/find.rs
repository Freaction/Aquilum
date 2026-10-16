use std::sync::mpsc::{Receiver, channel};

use aquilum_core::text::cancel::Generation;
use aquilum_core::text::matcher::Matcher;

use super::matching::{FIND, find_in};
use super::*;
use crate::ui::editor::change::{self, Change};

pub type Status = (usize, usize, bool);

#[derive(Default)]
pub(super) struct Find {
    query: String,
    matches: Vec<Range<usize>>,
    active: usize,
    only_active: bool,
    generation: Generation,
    pending: Option<Receiver<Vec<Range<usize>>>>,
}

impl TextEditor {
    fn find_status(&self) -> Status {
        let total = self.find.matches.len();
        (if total == 0 { 0 } else { self.find.active + 1 }, total, total == FIND.limit)
    }

    fn search(&mut self) {
        let cancel = self.find.generation.next();
        self.find.pending = None;
        let matcher = Matcher::new(&self.find.query, FIND);
        let (base, end) = self.doc.loaded_range();
        if matcher.is_empty() || base == 0 && end == self.doc.len() {
            self.find.matches = self.doc.search(|text| matcher.find_all(text));
        } else {
            let (tx, rx) = channel();
            let (rope, wake) = (self.doc.rope(), self.wake.clone());
            std::thread::spawn(move || {
                let _zone = aq_trace::zone("фон: поиск по заметке");
                if tx.send(find_in(&rope, &matcher, &cancel)).is_ok()
                    && let Some(wake) = wake
                {
                    wake();
                }
            });
            self.find.pending = Some(rx);
        }
        self.find.active = self.find.active.min(self.find.matches.len().saturating_sub(1));
    }

    pub(super) fn refind(&mut self, changes: &[Change]) {
        if self.find.query.is_empty() {
            return;
        }
        for found in &mut self.find.matches {
            *found = change::map(found.start, changes)..change::map(found.end, changes);
        }
        self.search();
    }

    pub(super) fn block_search(&self, block: Range<usize>) -> Option<Search> {
        if self.find.query.is_empty() {
            return None;
        }
        let query = self.find.query.clone();
        if !self.find.only_active {
            return Some(Search { query, only: None });
        }
        let active = self.find.matches.get(self.find.active)?;
        let before = self.find.matches.partition_point(|m| m.start < block.start);
        (block.start <= active.start && active.end <= block.end).then(|| Search { query, only: Some(self.find.active - before) })
    }

    pub(super) fn find_marks(&self, shown: &std::ops::RangeInclusive<usize>) -> &[Range<usize>] {
        let matches = &self.find.matches;
        match matches.get(self.find.active) {
            Some(active) if self.find.only_active => std::slice::from_ref(active),
            _ => {
                let visible = self.shown_bytes(shown);
                &matches[matches.partition_point(|m| m.end < visible.start)..matches.partition_point(|m| m.start <= visible.end)]
            }
        }
    }

    pub fn find(this: &mut WidgetMut<'_, Self>, query: &str) -> Status {
        let find = &mut this.widget.find;
        (find.query, find.active, find.only_active) = (query.to_owned(), 0, false);
        find.matches.clear();
        this.widget.search();
        this.ctx.request_render();
        this.widget.find_status()
    }

    pub fn poll_find(this: &mut WidgetMut<'_, Self>) -> Option<Status> {
        let found = this.widget.find.pending.as_ref()?.try_recv().ok()?;
        let find = &mut this.widget.find;
        find.pending = None;
        find.active = find.active.min(found.len().saturating_sub(1));
        find.matches = found;
        this.ctx.request_render();
        Some(this.widget.find_status())
    }

    pub fn find_step(this: &mut WidgetMut<'_, Self>, delta: isize) -> (Status, Option<usize>) {
        let total = this.widget.find.matches.len();
        if total > 0 {
            let find = &mut this.widget.find;
            find.active = crate::ui::keymap::cycle(find.active, total, delta > 0);
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
