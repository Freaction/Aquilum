use std::collections::HashMap;

use super::super::markdown::{self, Block};
use super::Document;

#[derive(Default)]
pub(super) struct States {
    list: Vec<markdown::State>,
    ids: HashMap<markdown::State, u32>,
}

impl States {
    fn id(&mut self, state: markdown::State) -> u32 {
        *self.ids.entry(state).or_insert_with(|| {
            self.list.push(state);
            (self.list.len() - 1) as u32
        })
    }

    fn get(&self, id: u32) -> Option<markdown::State> {
        self.list.get(id as usize).copied()
    }
}

impl Document {
    pub(super) fn classify(&mut self) {
        self.classify_from(0, self.paragraphs.len());
    }

    pub(super) fn classify_from(&mut self, from: usize, until: usize) {
        if self.markdown.is_none() {
            return;
        }
        let front = crate::frontmatter::end(&self.text).unwrap_or(0);
        let (from, until) = if front == self.front { (from, until) } else { (0, self.paragraphs.len()) };
        self.front = front;
        let mut state = from.checked_sub(1).and_then(|i| self.states.get(self.paragraphs[i].state)).unwrap_or_default();
        for j in from..self.paragraphs.len() {
            let line = &self.text[self.paragraphs[j].range()];
            let next = self.paragraphs.get(j + 1).map(|n| &self.text[n.range()]);
            let block = if self.paragraphs[j].start < front { Block::Meta } else { state.step(line, next) };
            let id = self.states.id(state);
            let p = &mut self.paragraphs[j];
            if p.block != block {
                p.block = block;
                p.forget();
            }
            let same = p.state == id;
            p.state = id;
            if j + 1 >= until && same {
                self.mark(from..j + 1);
                return;
            }
        }
        self.mark(from..self.paragraphs.len());
    }
}
