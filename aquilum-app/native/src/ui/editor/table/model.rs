pub const DEFAULT_ROW: f64 = 40.0;
pub const MIN_ROW: f64 = 28.0;
pub const DEFAULT_COL: f64 = 60.0;
pub const MIN_COL: f64 = 36.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Align {
    Left,
    Center,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Range2 {
    pub top: usize,
    pub left: usize,
    pub bottom: usize,
    pub right: usize,
}

impl Range2 {
    pub fn cell(r: usize, c: usize) -> Self {
        Range2 { top: r, left: c, bottom: r, right: c }
    }

    pub fn span(a: (usize, usize), b: (usize, usize)) -> Self {
        Range2 { top: a.0.min(b.0), left: a.1.min(b.1), bottom: a.0.max(b.0), right: a.1.max(b.1) }
    }

    pub fn contains(&self, r: usize, c: usize) -> bool {
        (self.top..=self.bottom).contains(&r) && (self.left..=self.right).contains(&c)
    }

    fn intersects(&self, o: &Range2) -> bool {
        self.top <= o.bottom && self.bottom >= o.top && self.left <= o.right && self.right >= o.left
    }

    fn covers(&self, o: &Range2) -> bool {
        self.top <= o.top && self.left <= o.left && self.bottom >= o.bottom && self.right >= o.right
    }

    pub fn single(&self) -> bool {
        self.top == self.bottom && self.left == self.right
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct Table {
    pub cells: Vec<Vec<String>>,
    pub merges: Vec<Range2>,
    pub aligns: Vec<Option<Align>>,
    pub heights: Vec<f64>,
    pub widths: Vec<f64>,
    pub explicit: bool,
}

impl Table {
    pub fn rows(&self) -> usize {
        self.cells.len()
    }

    pub fn cols(&self) -> usize {
        self.aligns.len()
    }

    pub(super) fn normalized(mut self) -> Self {
        let (rows, cols) = (self.rows(), self.cols());
        let mut merges: Vec<Range2> = Vec::new();
        for m in &self.merges {
            let r = Range2 { top: m.top, left: m.left, bottom: m.bottom.min(rows.saturating_sub(1)), right: m.right.min(cols.saturating_sub(1)) };
            if r.top > r.bottom || r.left > r.right || r.single() || merges.iter().any(|o| o.intersects(&r)) {
                continue;
            }
            merges.push(r);
        }
        merges.sort_by_key(|m| (m.top, m.left));
        self.merges = merges;
        self.sync();
        self
    }

    fn sync(&mut self) {
        for m in self.merges.clone() {
            let value = self.cells[m.top][m.left].clone();
            for r in m.top..=m.bottom {
                for c in m.left..=m.right {
                    self.cells[r][c] = value.clone();
                }
            }
        }
    }

    pub fn merge_at(&self, r: usize, c: usize) -> Option<Range2> {
        self.merges.iter().copied().find(|m| m.contains(r, c))
    }

    pub fn anchor(&self, r: usize, c: usize) -> (usize, usize) {
        self.merge_at(r, c).map_or((r, c), |m| (m.top, m.left))
    }

    pub fn region(&self, r: usize, c: usize) -> Range2 {
        self.merge_at(r, c).unwrap_or(Range2::cell(r, c))
    }

    pub fn value(&self, r: usize, c: usize) -> &str {
        let (r, c) = self.anchor(r, c);
        &self.cells[r][c]
    }

    pub fn expand(&self, mut range: Range2) -> Range2 {
        loop {
            let mut grown = range;
            for m in &self.merges {
                if m.intersects(&grown) {
                    grown = Range2 { top: grown.top.min(m.top), left: grown.left.min(m.left), bottom: grown.bottom.max(m.bottom), right: grown.right.max(m.right) };
                }
            }
            if grown == range {
                return range;
            }
            range = grown;
        }
    }

    pub fn set(&mut self, r: usize, c: usize, value: &str) {
        let region = self.region(r, c);
        let value: String = value.replace(['\n', '\r'], " ").replace('|', "\u{2223}").trim_end().to_owned();
        for rr in region.top..=region.bottom {
            for cc in region.left..=region.right {
                self.cells[rr][cc] = value.clone();
            }
        }
    }

    pub fn clear(&mut self, range: Range2) {
        for r in range.top..=range.bottom {
            for c in range.left..=range.right {
                self.cells[r][c].clear();
            }
        }
    }

    pub fn set_align(&mut self, c: usize, align: Option<Align>) {
        if let Some(a) = self.aligns.get_mut(c) {
            *a = align;
        }
    }

    pub fn can_merge(&self, range: Range2) -> bool {
        !range.single() && !self.merges.iter().any(|m| (range.intersects(m) && !range.covers(m)) || *m == range)
    }

    pub fn merge(&mut self, range: Range2) -> bool {
        if !self.can_merge(range) {
            return false;
        }
        self.merges.retain(|m| !m.intersects(&range));
        self.merges.push(range);
        self.merges.sort_by_key(|m| (m.top, m.left));
        self.sync();
        true
    }

    pub fn unmerge(&mut self, r: usize, c: usize) -> bool {
        let Some(m) = self.merge_at(r, c) else { return false };
        for rr in m.top..=m.bottom {
            for cc in m.left..=m.right {
                if (rr, cc) != (m.top, m.left) {
                    self.cells[rr][cc].clear();
                }
            }
        }
        self.merges.retain(|x| *x != m);
        true
    }

    pub fn add_row(&mut self) {
        let cols = self.cols();
        self.cells.push(vec![String::new(); cols]);
        self.heights.push(DEFAULT_ROW);
    }

    pub fn add_col(&mut self) {
        for row in &mut self.cells {
            row.push(String::new());
        }
        self.aligns.push(None);
        self.widths.push(DEFAULT_COL);
    }

    pub fn delete_row(&mut self, index: usize) -> bool {
        if index >= self.rows() || self.rows() <= 2 {
            return false;
        }
        let values: Vec<(Range2, String)> = self.merges.iter().map(|m| (*m, self.cells[m.top][m.left].clone())).collect();
        self.cells.remove(index);
        self.heights.remove(index);
        let mut merges = Vec::new();
        for (m, v) in values {
            let r = if index < m.top {
                Range2 { top: m.top - 1, bottom: m.bottom - 1, ..m }
            } else if index <= m.bottom {
                if m.top == m.bottom {
                    continue;
                }
                Range2 { bottom: m.bottom - 1, ..m }
            } else {
                m
            };
            if !r.single() {
                self.cells[r.top][r.left] = v;
                merges.push(r);
            }
        }
        self.merges = merges;
        self.sync();
        true
    }

    pub fn delete_col(&mut self, index: usize) -> bool {
        if index >= self.cols() || self.cols() <= 1 {
            return false;
        }
        let values: Vec<(Range2, String)> = self.merges.iter().map(|m| (*m, self.cells[m.top][m.left].clone())).collect();
        for row in &mut self.cells {
            row.remove(index);
        }
        self.aligns.remove(index);
        self.widths.remove(index);
        let mut merges = Vec::new();
        for (m, v) in values {
            let r = if index < m.left {
                Range2 { left: m.left - 1, right: m.right - 1, ..m }
            } else if index <= m.right {
                if m.left == m.right {
                    continue;
                }
                Range2 { right: m.right - 1, ..m }
            } else {
                m
            };
            if !r.single() {
                self.cells[r.top][r.left] = v;
                merges.push(r);
            }
        }
        self.merges = merges;
        self.sync();
        true
    }

    fn order(len: usize, start: usize, end: usize, dest: usize) -> Option<Vec<usize>> {
        let size = end - start + 1;
        let dest = dest.min(len - size);
        if dest == start {
            return None;
        }
        let mut order: Vec<usize> = (0..len).collect();
        let block: Vec<usize> = order.drain(start..=end).collect();
        order.splice(dest..dest, block);
        Some(order)
    }

    pub fn move_rows(&mut self, top: usize, bottom: usize, dest: usize) -> bool {
        let Some(order) = Self::order(self.rows(), top, bottom, dest) else { return false };
        let mut back = vec![0; order.len()];
        for (new, old) in order.iter().enumerate() {
            back[*old] = new;
        }
        self.cells = order.iter().map(|o| self.cells[*o].clone()).collect();
        self.heights = order.iter().map(|o| self.heights[*o]).collect();
        self.merges = self.merges.iter().map(|m| Range2 { top: back[m.top], bottom: back[m.bottom], ..*m }).collect();
        *self = self.clone().normalized();
        true
    }

    pub fn move_cols(&mut self, left: usize, right: usize, dest: usize) -> bool {
        let Some(order) = Self::order(self.cols(), left, right, dest) else { return false };
        let mut back = vec![0; order.len()];
        for (new, old) in order.iter().enumerate() {
            back[*old] = new;
        }
        self.cells = self.cells.iter().map(|row| order.iter().map(|o| row[*o].clone()).collect()).collect();
        self.widths = order.iter().map(|o| self.widths[*o]).collect();
        self.aligns = order.iter().map(|o| self.aligns[*o]).collect();
        self.merges = self.merges.iter().map(|m| Range2 { left: back[m.left], right: back[m.right], ..*m }).collect();
        *self = self.clone().normalized();
        true
    }
}
