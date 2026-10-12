use super::model::{Range2, Table};

impl Table {
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

    pub(super) fn sync(&mut self) {
        for m in self.merges.clone() {
            let value = self.cells[m.top][m.left].clone();
            for r in m.top..=m.bottom {
                for c in m.left..=m.right {
                    self.cells[r][c] = value.clone();
                }
            }
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
}
