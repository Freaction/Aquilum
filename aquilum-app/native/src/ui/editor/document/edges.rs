use super::Document;

impl Document {
    pub(super) fn reach(&mut self, a: usize, b: usize) {
        if a > self.base {
            self.drop_front(a);
        }
        if b < self.end() {
            self.drop_back(b);
        }
        if b > self.end() {
            self.append(b);
        }
        if a < self.base {
            self.prepend(a);
        }
    }

    fn drop_front(&mut self, a: usize) {
        let (d, bytes) = (self.lines.at(a - self.base), a - self.base);
        self.above += self.heights.top(d);
        self.entry = self.blocks.state(d - 1).unwrap_or(self.entry);
        self.text.drain(..bytes);
        self.lines.drop_front(d, bytes);
        self.blocks.drop_front(d);
        self.heights.drop_front(d);
        self.laid.splice(0..=d - 1, 0);
        self.embeds.splice(0..=d - 1, 0);
        self.dirty = self.dirty.take().map(|r| r.start.saturating_sub(d)..r.end.saturating_sub(d)).filter(|r| !r.is_empty());
        self.base = a;
    }

    fn drop_back(&mut self, b: usize) {
        let keep = self.lines.at(b - self.base) + 1;
        self.below += self.heights.total() - self.heights.top(keep);
        self.text.truncate(b - self.base);
        self.lines.truncate(keep, b - self.base);
        self.blocks.truncate(keep);
        self.heights.truncate(keep);
        self.laid.retain(|i| i < keep);
        self.embeds.retain(|i| i < keep);
        self.dirty = self.dirty.take().map(|r| r.start.min(keep)..r.end.min(keep)).filter(|r| !r.is_empty());
    }

    fn append(&mut self, b: usize) {
        let added = self.full.byte_slice(self.end()..b).to_string();
        let old = self.lines.count();
        self.lines.extend(&added, self.text.len());
        self.text.push_str(&added);
        let fresh = self.lines.count() - old;
        self.blocks.extend(fresh);
        self.heights.extend(fresh);
        let sum = self.estimated(old..old + fresh);
        self.below = if b < self.len() { (self.below - sum).max(0.0) } else { 0.0 };
        self.mark(old - 1..old + fresh);
        self.classify_from(old - 1, old + fresh);
    }

    fn prepend(&mut self, a: usize) {
        let added = self.full.byte_slice(a..self.base).to_string();
        let fresh = self.lines.prepend(&added);
        self.text.insert_str(0, &added);
        self.blocks.prepend(fresh);
        self.heights.prepend(fresh);
        self.laid.shift(fresh);
        self.embeds.shift(fresh);
        self.dirty = self.dirty.take().map(|r| r.start + fresh..r.end + fresh);
        self.base = a;
        let sum = self.estimated(0..fresh);
        self.above = if a > 0 { (self.above - sum).max(0.0) } else { 0.0 };
        self.entry = self.checkpoint(a);
        self.mark(0..fresh + 1);
        self.classify_from(0, fresh + 1);
    }
}
