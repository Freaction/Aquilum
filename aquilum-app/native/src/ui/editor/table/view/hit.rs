use masonry::kurbo::Point;

use super::{ADD, GAP, Hit, Line, PAD_Y, RESIZE_ZONE, TOP, TableView};

impl TableView {
    pub fn hit(&self, p: Point, scroll_x: f64) -> Hit {
        let (n, m) = (self.ys.len() - 1, self.xs.len() - 1);
        let tx = self.table_x(scroll_x);
        let table_bottom = TOP + self.ys[n];
        if self.bar_track().is_some_and(|t| t.contains(p)) {
            return Hit::Bar;
        }
        if p.y < PAD_Y - RESIZE_ZONE || p.y > table_bottom + ADD + RESIZE_ZONE {
            return Hit::Outside;
        }
        if p.x >= self.edge() && p.x <= self.edge() + ADD && p.y >= PAD_Y && p.y <= table_bottom {
            return Hit::AddCol;
        }
        if p.x > self.edge() {
            return Hit::Empty;
        }
        let on_row_strip = p.x < self.corner;
        let on_col_strip = p.y < TOP - GAP;
        let row_border = (1..=n).map(|r| (r, (p.y - (TOP + self.ys[r])).abs())).filter(|(_, d)| *d <= RESIZE_ZONE).min_by(|a, b| a.1.total_cmp(&b.1));
        let col_border = (1..=m).map(|c| (c, (p.x - (tx + self.xs[c])).abs())).filter(|(_, d)| *d <= RESIZE_ZONE).min_by(|a, b| a.1.total_cmp(&b.1));
        match (row_border, col_border) {
            (Some((r, dr)), Some((c, dc))) => {
                if dr <= dc && !on_col_strip {
                    return Hit::RowBorder(r - 1);
                }
                if !on_row_strip {
                    return Hit::ColumnBorder(c - 1);
                }
            }
            (Some((r, _)), None) if !on_col_strip && p.y <= table_bottom + RESIZE_ZONE => return Hit::RowBorder(r - 1),
            (None, Some((c, _))) if !on_row_strip && p.y <= table_bottom => return Hit::ColumnBorder(c - 1),
            _ => {}
        }
        if p.y > table_bottom && p.y <= table_bottom + ADD {
            return Hit::AddRow;
        }
        if p.y >= PAD_Y && on_col_strip && p.x >= tx {
            return (0..m).find(|c| p.x >= tx + self.xs[*c] && p.x < tx + self.xs[c + 1]).map_or(Hit::Empty, Hit::Column);
        }
        if on_row_strip && p.y >= TOP && p.y < table_bottom {
            return (0..n).find(|r| p.y >= TOP + self.ys[*r] && p.y < TOP + self.ys[r + 1]).map_or(Hit::Empty, Hit::Row);
        }
        if p.x >= tx && p.y >= TOP && p.y < table_bottom {
            let c = (0..m).find(|c| p.x >= tx + self.xs[*c] && p.x < tx + self.xs[c + 1]);
            let r = (0..n).find(|r| p.y >= TOP + self.ys[*r] && p.y < TOP + self.ys[r + 1]);
            if let (Some(r), Some(c)) = (r, c) {
                let (ar, ac) = self.model.anchor(r, c);
                if let Some(cell) = self.cell(ar, ac) {
                    return Hit::Cell(ar, ac, p - self.cell_origin(cell, scroll_x).to_vec2());
                }
            }
        }
        Hit::Empty
    }

    pub fn gap(&self, along: f64, rows: bool, scroll_x: f64) -> usize {
        if rows {
            let n = self.ys.len() - 1;
            (0..n).find(|r| along < TOP + (self.ys[*r] + self.ys[r + 1]) / 2.0).unwrap_or(n)
        } else {
            let m = self.xs.len() - 1;
            let tx = self.table_x(scroll_x);
            (0..m).find(|c| along < tx + (self.xs[*c] + self.xs[c + 1]) / 2.0).unwrap_or(m)
        }
    }

    pub fn gap_line(&self, gap: usize, rows: bool, scroll_x: f64, valid: bool) -> Line {
        if rows { Line::Row(TOP + self.ys[gap], valid) } else { Line::Col(self.table_x(scroll_x) + self.xs[gap], valid) }
    }

    pub fn border_line(&self, hit: Hit, scroll_x: f64) -> Line {
        match hit {
            Hit::RowBorder(r) => Line::Row(TOP + self.ys[r + 1], true),
            Hit::ColumnBorder(c) => Line::Col(self.table_x(scroll_x) + self.xs[c + 1], true),
            _ => Line::None,
        }
    }
}
