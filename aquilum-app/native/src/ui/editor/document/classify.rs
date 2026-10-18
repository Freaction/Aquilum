use std::borrow::Cow;
use std::collections::HashMap;
use std::hash::Hash;
use std::ops::RangeInclusive;

use foldhash::fast::FixedState;

use super::super::markdown::{self, Block, Fonts, Frame};
use super::Document;

struct Interned<T> {
    list: Vec<T>,
    ids: HashMap<T, u32, FixedState>,
    last: u32,
}

impl<T: Clone + Eq + Hash> Interned<T> {
    fn new(first: T) -> Self {
        let mut ids = HashMap::default();
        ids.insert(first.clone(), 0);
        Interned { list: vec![first], ids, last: 0 }
    }

    fn id(&mut self, value: T) -> u32 {
        if self.list[self.last as usize] != value {
            self.last = match self.ids.get(&value) {
                Some(&id) => id,
                None => {
                    self.list.push(value.clone());
                    self.ids.insert(value, (self.list.len() - 1) as u32);
                    (self.list.len() - 1) as u32
                }
            };
        }
        self.last
    }
}

pub(super) struct Blocks {
    states: Vec<u32>,
    kinds: Vec<u32>,
    state_ids: Interned<markdown::State>,
    block_ids: Interned<(Frame, Block)>,
}

impl Blocks {
    pub(super) fn new(count: usize) -> Self {
        Blocks { states: vec![u32::MAX; count], kinds: vec![0; count], state_ids: Interned::new(markdown::State::default()), block_ids: Interned::new((Frame::default(), Block::Text)) }
    }

    pub(super) fn get(&self, line: usize) -> &(Frame, Block) {
        &self.block_ids.list[self.kinds[line] as usize]
    }

    pub(super) fn state(&self, line: usize) -> Option<markdown::State> {
        self.state_ids.list.get(self.states[line] as usize).copied()
    }

    fn set(&mut self, line: usize, state: markdown::State, block: (Frame, Block)) -> (bool, bool) {
        let (state, kind) = (self.state_ids.id(state), self.block_ids.id(block));
        let changed = std::mem::replace(&mut self.kinds[line], kind) != kind;
        let same = std::mem::replace(&mut self.states[line], state) == state;
        (changed, same)
    }

    pub(super) fn splice(&mut self, lines: RangeInclusive<usize>, count: usize) {
        self.states.splice(lines.clone(), std::iter::repeat_n(u32::MAX, count));
        self.kinds.splice(lines, std::iter::repeat_n(0, count));
    }

    pub(super) fn drop_front(&mut self, lines: usize) {
        self.states.drain(..lines);
        self.kinds.drain(..lines);
    }

    pub(super) fn truncate(&mut self, keep: usize) {
        self.states.truncate(keep);
        self.kinds.truncate(keep);
    }

    pub(super) fn extend(&mut self, count: usize) {
        self.states.extend(std::iter::repeat_n(u32::MAX, count));
        self.kinds.extend(std::iter::repeat_n(0, count));
    }

    pub(super) fn prepend(&mut self, count: usize) {
        self.states.splice(0..0, std::iter::repeat_n(u32::MAX, count));
        self.kinds.splice(0..0, std::iter::repeat_n(0, count));
    }

    #[cfg(feature = "memory-probe")]
    pub(super) fn bytes(&self) -> usize {
        (self.states.capacity() + self.kinds.capacity()) * 4
    }
}

impl Document {
    pub fn set_flavors(&mut self, callouts: bool, tags: bool) {
        if let Some(fonts) = self.markdown.as_mut()
            && (fonts.callouts != callouts || fonts.tags != tags)
        {
            fonts.callouts = callouts;
            fonts.tags = tags;
            self.invalidate_all();
        }
    }

    pub fn set_markdown(&mut self, fonts: Option<Fonts>) {
        let _zone = aq_trace::zone("документ: разбор блоков");
        self.markdown = fonts;
        self.invalidate_all();
        self.classify();
    }

    pub(super) fn classify(&mut self) {
        self.classify_from(0, self.lines.count());
    }

    fn following(&self, j: usize) -> Option<Cow<'_, str>> {
        if j + 1 < self.lines.count() {
            return Some(Cow::Borrowed(&self.text[self.lines.range(j + 1)]));
        }
        (self.end() < self.len()).then(|| Cow::Owned(self.full.line(self.line_of(self.end() + 1)).chunks().collect()))
    }

    pub(super) fn classify_from(&mut self, from: usize, until: usize) {
        if self.markdown.is_none() {
            return;
        }
        let count = self.lines.count();
        let front = if self.base == 0 { crate::frontmatter::end(&self.text).unwrap_or(0) } else { 0 };
        let (from, until) = if front == self.front { (from, until) } else { (0, count) };
        self.front = front;
        let mut state = from.checked_sub(1).and_then(|i| self.blocks.state(i)).unwrap_or(self.entry);
        for j in from..count {
            let line = &self.text[self.lines.range(j)];
            let next = self.following(j);
            let block = if self.lines.start(j) >= front {
                state.step(line, next.as_deref())
            } else if crate::frontmatter::fence(line) {
                (Frame::default(), Block::Rule)
            } else {
                (Frame::default(), Block::Meta)
            };
            drop(next);
            let (changed, same) = self.blocks.set(j, state, block);
            if changed {
                self.laid.remove(j);
            }
            if j + 1 >= until && same {
                self.mark(from..j + 1);
                return;
            }
        }
        self.mark(from..count);
    }
}
