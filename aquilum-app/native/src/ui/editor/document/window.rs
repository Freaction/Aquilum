use super::{Document, classify, heights, lines};

const MARGIN: usize = 3000;
const JUMP: usize = 256;
const SPAN_BYTES: usize = 512 * 1024;

impl Document {
    fn back(&self, from: usize, lines: usize) -> usize {
        let line = self.line_of(from);
        let by_bytes = self.line_start(self.line_of(from.saturating_sub(SPAN_BYTES)));
        self.line_start(line.saturating_sub(lines)).max(by_bytes)
    }

    fn forth(&self, from: usize, lines: usize) -> usize {
        let line = self.line_of(from);
        let last = self.count() - 1;
        let by_bytes = self.line_end(self.line_of((from + SPAN_BYTES).min(self.len())));
        self.line_end(line.saturating_add(lines).min(last)).min(by_bytes)
    }

    pub fn y_of(&self, offset: usize) -> f64 {
        let (len, end, inner) = (self.len(), self.end(), self.heights.total());
        match offset {
            o if o < self.base => self.above * o as f64 / self.base as f64,
            o if o > end && len > end => self.above + inner + self.below * (o - end) as f64 / (len - end) as f64,
            o => self.top(self.at_offset(o)),
        }
    }

    fn byte_at(&self, y: f64) -> usize {
        let (len, end, inner) = (self.len(), self.end(), self.heights.total());
        if y < self.above {
            return (self.base as f64 * (y / self.above).clamp(0.0, 1.0)) as usize;
        }
        let below = if self.below > 0.0 { ((y - self.above - inner) / self.below).clamp(0.0, 1.0) } else { 0.0 };
        end + ((len - end) as f64 * below) as usize
    }

    pub fn track(&mut self, view: (f64, f64)) -> bool {
        self.view = view;
        let before = (self.base, self.end(), self.entry);
        self.refresh_entry();
        self.follow();
        (self.base, self.end(), self.entry) != before
    }

    fn follow(&mut self) {
        if !self.windowed {
            return;
        }
        let (top, bottom) = self.view;
        if bottom <= top || !self.placed {
            return;
        }
        if bottom < self.above || top > self.above + self.heights.total() {
            let byte = self.byte_at(top);
            self.load(byte, Some(top));
            return;
        }
        let (first, last, count) = (self.at_y(top), self.at_y(bottom), self.lines.count());
        if first < MARGIN / 2 && self.base > 0 || count - 1 - last < MARGIN / 2 && self.end() < self.len() {
            let a = self.back(self.start(first), MARGIN).max(self.back(self.base, JUMP));
            let b = self.forth(self.start(last), MARGIN).min(self.forth(self.end(), JUMP));
            self.reach(a, b);
        }
    }

    pub(super) fn load(&mut self, byte: usize, align: Option<f64>) {
        let start = self.line_start(self.line_of(byte.min(self.len())));
        let reach = if align.is_some() { JUMP } else { MARGIN };
        let (a, b) = if self.windowed { (self.back(start, reach), self.forth(start, reach)) } else { (0, self.len()) };
        let total = self.height();
        self.base = a;
        self.text = self.full.byte_slice(a..b).to_string();
        self.lines = lines::Lines::new(&self.text);
        let count = self.lines.count();
        self.blocks = classify::Blocks::new(count);
        self.heights = heights::Heights::new(count);
        self.laid.clear();
        self.embeds.clear();
        self.entry = self.checkpoint(a);
        (self.placed, self.anchor, self.dirty) = (false, None, Some(0..count));
        self.above = self.guess(0..a);
        self.below = self.guess(b..self.len());
        self.jump = align.map(|y| (self.lines.at(start - a), y, total));
        self.classify();
    }

    pub(super) fn aligned(&mut self) -> bool {
        let Some((line, y, total)) = self.jump.take() else { return false };
        let inner = self.heights.total();
        self.above = if self.base > 0 { (y - self.heights.top(line)).max(0.0) } else { 0.0 };
        self.below = if self.end() < self.len() { (total - self.above - inner).max(0.0) } else { 0.0 };
        true
    }

    pub(super) fn reguess(&mut self) {
        if self.windowed {
            self.above = self.guess(0..self.base);
            self.below = self.guess(self.end()..self.len());
        }
    }
}
