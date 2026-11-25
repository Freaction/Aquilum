use super::model::{DEFAULT_COL, DEFAULT_ROW, Range2, Table};

fn shrink(start: usize, end: usize, index: usize) -> Option<(usize, usize)> {
    if index < start {
        Some((start - 1, end - 1))
    } else if index > end {
        Some((start, end))
    } else if start == end {
        None
    } else {
        Some((start, end - 1))
    }
}

impl Table {
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
        self.delete_line(
            |table| {
                table.cells.remove(index);
                table.heights.remove(index);
            },
            |m| shrink(m.top, m.bottom, index).map(|(top, bottom)| Range2 { top, bottom, ..m }),
        );
        true
    }

    pub fn delete_col(&mut self, index: usize) -> bool {
        if index >= self.cols() || self.cols() <= 1 {
            return false;
        }
        self.delete_line(
            |table| {
                for row in &mut table.cells {
                    row.remove(index);
                }
                table.aligns.remove(index);
                table.widths.remove(index);
            },
            |m| shrink(m.left, m.right, index).map(|(left, right)| Range2 { left, right, ..m }),
        );
        true
    }

    fn delete_line(&mut self, remove: impl FnOnce(&mut Self), moved: impl Fn(Range2) -> Option<Range2>) {
        let values: Vec<(Range2, String)> = self.merges.iter().map(|m| (*m, self.cells[m.top][m.left].clone())).collect();
        remove(self);
        let mut merges = Vec::new();
        for (m, v) in values {
            let Some(r) = moved(m).filter(|r| !r.single()) else { continue };
            self.cells[r.top][r.left] = v;
            merges.push(r);
        }
        self.merges = merges;
        self.sync();
    }

    fn order(len: usize, start: usize, end: usize, dest: usize) -> Option<(Vec<usize>, Vec<usize>)> {
        let size = end - start + 1;
        let dest = dest.min(len - size);
        if dest == start {
            return None;
        }
        let mut order: Vec<usize> = (0..len).collect();
        let block: Vec<usize> = order.drain(start..=end).collect();
        order.splice(dest..dest, block);
        let mut back = vec![0; len];
        for (new, old) in order.iter().enumerate() {
            back[*old] = new;
        }
        Some((order, back))
    }

    pub fn move_rows(&mut self, top: usize, bottom: usize, dest: usize) -> bool {
        let Some((order, back)) = Self::order(self.rows(), top, bottom, dest) else { return false };
        self.cells = order.iter().map(|o| self.cells[*o].clone()).collect();
        self.heights = order.iter().map(|o| self.heights[*o]).collect();
        self.merges = self.merges.iter().map(|m| Range2 { top: back[m.top], bottom: back[m.bottom], ..*m }).collect();
        *self = self.clone().normalized();
        true
    }

    pub fn move_cols(&mut self, left: usize, right: usize, dest: usize) -> bool {
        let Some((order, back)) = Self::order(self.cols(), left, right, dest) else { return false };
        self.cells = self.cells.iter().map(|row| order.iter().map(|o| row[*o].clone()).collect()).collect();
        self.widths = order.iter().map(|o| self.widths[*o]).collect();
        self.aligns = order.iter().map(|o| self.aligns[*o]).collect();
        self.merges = self.merges.iter().map(|m| Range2 { left: back[m.left], right: back[m.right], ..*m }).collect();
        *self = self.clone().normalized();
        true
    }
}
