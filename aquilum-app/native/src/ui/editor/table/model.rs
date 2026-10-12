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

    pub(super) fn intersects(&self, o: &Range2) -> bool {
        self.top <= o.bottom && self.bottom >= o.top && self.left <= o.right && self.right >= o.left
    }

    pub(super) fn covers(&self, o: &Range2) -> bool {
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
    pub fn empty(cols: usize, rows: usize) -> Self {
        Table { cells: vec![vec![String::new(); cols]; rows], merges: Vec::new(), aligns: vec![None; cols], heights: vec![DEFAULT_ROW; rows], widths: vec![DEFAULT_COL; cols], explicit: false }
    }

    pub fn rows(&self) -> usize {
        self.cells.len()
    }

    pub fn cols(&self) -> usize {
        self.aligns.len()
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
        let value = value.replace(['\n', '\r'], " ");
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
}
