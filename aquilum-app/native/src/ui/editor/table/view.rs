use masonry::core::{BrushIndex, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, BezPath, Point, Rect, RoundedRect, Stroke, Vec2};
use masonry::parley::{Alignment, Layout};
use masonry::peniko::Brush;

use super::model::{Align, Range2, Table};
use crate::ui::scroll::bar;
use crate::ui::theme;
use crate::ui::tokens::size;

const PAD_Y: f64 = 12.0;
const STRIP: f64 = 24.0;
const STRIP_PAD: f64 = 4.0;
const ADD: f64 = 20.0;
const GAP: f64 = 2.0;
const RESIZE_ZONE: f64 = 6.0;
const PLUS: f64 = 16.0;
pub const TOP: f64 = PAD_Y + STRIP + GAP;

pub fn letters(mut n: usize) -> String {
    let mut out = Vec::new();
    loop {
        out.push(b'A' + (n % 26) as u8);
        if n < 26 {
            break;
        }
        n = n / 26 - 1;
    }
    out.reverse();
    String::from_utf8(out).unwrap_or_default()
}

pub fn cell_label(value: &str) -> (String, bool) {
    let v = value.trim();
    if let Some(inner) = v.strip_prefix("[[").and_then(|x| x.strip_suffix("]]"))
        && !inner.contains("]]")
    {
        let shown = inner.rsplit('|').next().unwrap_or(inner);
        return (shown.to_owned(), true);
    }
    (value.to_owned(), false)
}

pub struct Cell {
    pub row: usize,
    pub col: usize,
    pub rect: Rect,
    pub layout: Layout<BrushIndex>,
    pub link: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Hit {
    Cell(usize, usize, Point),
    Column(usize),
    Row(usize),
    ColumnBorder(usize),
    RowBorder(usize),
    AddRow,
    AddCol,
    Bar,
    Empty,
    Outside,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum Line {
    #[default]
    None,
    Row(f64, bool),
    Col(f64, bool),
}

#[derive(Default)]
pub struct Overlay {
    pub selection: Option<Range2>,
    pub hover: Option<Hit>,
    pub line: Line,
    pub scroll_x: f64,
    pub hovered: bool,
}

pub struct TableView {
    pub model: Table,
    pub height: f64,
    pub corner: f64,
    pub scroll_w: f64,
    pub layout_w: f64,
    pub xs: Vec<f64>,
    pub ys: Vec<f64>,
    pub cells: Vec<Cell>,
    columns: Vec<Layout<BrushIndex>>,
    numbers: Vec<Layout<BrushIndex>>,
}

impl Table {
    pub fn layout(&self, available: f64, editing: Option<(usize, usize)>, mut make: impl FnMut(&str, Option<f64>, Alignment, bool) -> Layout<BrushIndex>) -> TableView {
        let (n, m) = (self.rows(), self.cols());
        let numbers: Vec<Layout<BrushIndex>> = (0..n).map(|r| make(&(r + 1).to_string(), None, Alignment::Start, false)).collect();
        let columns: Vec<Layout<BrushIndex>> = (0..m).map(|c| make(&letters(c), None, Alignment::Start, false)).collect();
        let corner = numbers.iter().map(|l| f64::from(l.width()) + STRIP_PAD * 2.0).fold(STRIP, f64::max);
        let scroll_w = (available - ADD).max(STRIP * 2.0);
        let widths: Vec<f64> = if self.explicit { self.widths.clone() } else { vec![((scroll_w - corner - GAP) / m as f64).max(1.0); m] };
        let mut xs = vec![0.0];
        for w in &widths {
            xs.push(xs.last().copied().unwrap_or(0.0) + w);
        }
        let layout_w = (xs[m] + corner + GAP).max(scroll_w);
        let mut heights = self.heights.clone();
        let pad = size::EDITOR_TABLE_CELL_PADDING;
        let mut placed = Vec::new();
        for r in 0..n {
            for c in 0..m {
                let region = self.region(r, c);
                if (region.top, region.left) != (r, c) {
                    continue;
                }
                let width = xs[region.right + 1] - xs[c];
                let (text, link) = if editing == Some((r, c)) { (self.cells[r][c].clone(), false) } else { cell_label(&self.cells[r][c]) };
                let align = match self.aligns[c] {
                    Some(Align::Center) => Alignment::Center,
                    Some(Align::Right) => Alignment::End,
                    _ => Alignment::Start,
                };
                let layout = make(&text, Some((width - pad * 2.0).max(1.0)), align, link);
                let need = f64::from(layout.height()) + pad * 2.0;
                let have: f64 = heights[r..=region.bottom].iter().sum();
                if need > have {
                    heights[region.bottom] += need - have;
                }
                placed.push((r, c, region, layout, link));
            }
        }
        let mut ys = vec![0.0];
        for h in &heights {
            ys.push(ys.last().copied().unwrap_or(0.0) + h);
        }
        let cells = placed
            .into_iter()
            .map(|(row, col, region, layout, link)| Cell { row, col, rect: Rect::new(xs[col], ys[row], xs[region.right + 1], ys[region.bottom + 1]), layout, link })
            .collect();
        let bar = if layout_w > scroll_w { size::SCROLLBAR_WIDTH } else { 0.0 };
        TableView { model: self.clone(), height: TOP + ys[n] + ADD + bar + PAD_Y, corner, scroll_w, layout_w, xs, ys, cells, columns, numbers }
    }
}

impl TableView {
    pub fn max_scroll(&self) -> f64 {
        (self.layout_w - self.scroll_w).max(0.0)
    }

    pub fn bar_track(&self) -> Option<Rect> {
        let bottom = TOP + self.ys[self.ys.len() - 1] + ADD;
        (self.max_scroll() > 0.0).then(|| Rect::new(0.0, bottom, self.scroll_w, bottom + size::SCROLLBAR_WIDTH))
    }

    pub fn bar_thumb(&self, scroll_x: f64) -> Option<Rect> {
        let t = self.bar_track()?.inflate(-bar::THUMB_INSET, -bar::THUMB_INSET);
        let width = (t.width() * self.scroll_w / self.layout_w).clamp(bar::THUMB_MIN.min(t.width()), t.width());
        let left = t.x0 + (t.width() - width) * scroll_x.clamp(0.0, self.max_scroll()) / self.max_scroll();
        Some(Rect::new(left, t.y0, left + width, t.y1))
    }

    pub fn bar_travel(&self) -> f64 {
        match (self.bar_track(), self.bar_thumb(0.0)) {
            (Some(track), Some(thumb)) => (track.width() - bar::THUMB_INSET * 2.0 - thumb.width()).max(1.0),
            _ => 1.0,
        }
    }

    fn table_x(&self, scroll_x: f64) -> f64 {
        self.corner + GAP - scroll_x
    }

    pub fn cell_origin(&self, cell: &Cell, scroll_x: f64) -> Point {
        let pad = size::EDITOR_TABLE_CELL_PADDING;
        Point::new(self.table_x(scroll_x) + cell.rect.x0 + pad, TOP + cell.rect.y0 + pad)
    }

    pub fn cell(&self, r: usize, c: usize) -> Option<&Cell> {
        let (r, c) = self.model.anchor(r, c);
        self.cells.iter().find(|cell| cell.row == r && cell.col == c)
    }

    pub fn range_rect(&self, range: Range2, scroll_x: f64) -> Rect {
        let x = self.table_x(scroll_x);
        Rect::new(x + self.xs[range.left], TOP + self.ys[range.top], x + self.xs[range.right + 1], TOP + self.ys[range.bottom + 1])
    }

    pub fn widths(&self) -> Vec<f64> {
        self.xs.windows(2).map(|w| w[1] - w[0]).collect()
    }

    pub fn heights(&self) -> Vec<f64> {
        self.ys.windows(2).map(|w| w[1] - w[0]).collect()
    }

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
        if p.x >= self.scroll_w && p.x <= self.scroll_w + ADD && p.y >= PAD_Y && p.y <= table_bottom {
            return Hit::AddCol;
        }
        if p.x > self.scroll_w {
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

    pub fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, palette: &[Brush], overlay: &Overlay) {
        let tm = theme::current();
        let s = overlay.scroll_x.clamp(0.0, self.max_scroll());
        let n = self.ys.len() - 1;
        let tx = self.table_x(s);
        let bottom = TOP + self.ys[n];
        let stroke = Stroke::new(size::BORDER_1);
        let centered = |rect: Rect, label: &Layout<BrushIndex>| Affine::translate((x + rect.center().x - f64::from(label.width()) / 2.0, y + rect.center().y - f64::from(label.height()) / 2.0));
        painter.with_fill_clip(Rect::new(x + self.corner, y, x + self.scroll_w, y + self.height), |painter| {
            for (c, label) in self.columns.iter().enumerate() {
                let rect = Rect::new(tx + self.xs[c], PAD_Y, tx + self.xs[c + 1], PAD_Y + STRIP);
                if overlay.hover == Some(Hit::Column(c)) {
                    painter.fill(RoundedRect::from_rect(rect + Vec2::new(x, y), 2.0), tm.bg_surface_hover).draw();
                }
                render_text(painter, centered(rect, label), label, palette, true);
            }
            for cell in &self.cells {
                let r = Rect::new(tx + cell.rect.x0, TOP + cell.rect.y0, tx + cell.rect.x1, TOP + cell.rect.y1);
                let px = Rect::new(x + r.x0.round() + 0.5, y + r.y0.round() + 0.5, x + r.x1.round() + 0.5, y + r.y1.round() + 0.5);
                painter.stroke(px, &stroke, tm.editor_table_border).draw();
                let o = self.cell_origin(cell, s);
                render_text(painter, Affine::translate((x + o.x, y + o.y)), &cell.layout, palette, true);
            }
            if let Some(range) = overlay.selection {
                let r = self.range_rect(range, s) + Vec2::new(x, y);
                painter.fill(r, tm.editor_table_selection_bg).draw();
                painter.stroke(r.inset(-1.0), &Stroke::new(2.0), tm.border_accent).draw();
            }
        });
        painter.with_fill_clip(Rect::new(x, y + TOP, x + self.corner, y + bottom), |painter| {
            for (r, label) in self.numbers.iter().enumerate() {
                let rect = Rect::new(0.0, TOP + self.ys[r], self.corner, TOP + self.ys[r + 1]);
                if overlay.hover == Some(Hit::Row(r)) {
                    painter.fill(RoundedRect::from_rect(rect + Vec2::new(x, y), 2.0), tm.bg_surface_hover).draw();
                }
                render_text(painter, centered(rect, label), label, palette, true);
            }
        });
        if overlay.hovered {
            let row = Rect::new(x, y + bottom, x + self.layout_w.min(self.scroll_w), y + bottom + ADD);
            let col = Rect::new(x + self.scroll_w, y + PAD_Y, x + self.scroll_w + ADD, y + bottom);
            for band in [row, col] {
                painter.fill(band, tm.editor_table_add_bg).draw();
                painter.stroke(band.inset(-0.5), &stroke, tm.editor_table_add_border).draw();
                plus(painter, band.center(), tm.icon_faint);
            }
        }
        if let (Some(track), Some(thumb)) = (self.bar_track(), self.bar_thumb(s)) {
            let at = Vec2::new(x, y);
            bar::paint_scrollbar(painter, track + at, thumb + at, overlay.hover == Some(Hit::Bar));
        }
        let (valid, invalid) = (tm.bg_accent, tm.text_muted.multiply_alpha(0.6));
        match overlay.line {
            Line::Row(ly, ok) => painter.fill(RoundedRect::new(x, y + ly - 1.0, x + self.layout_w.min(self.scroll_w), y + ly + 1.0, 1.0), if ok { valid } else { invalid }).draw(),
            Line::Col(lx, ok) => painter.fill(RoundedRect::new(x + lx - 1.0, y + PAD_Y, x + lx + 1.0, y + bottom, 1.0), if ok { valid } else { invalid }).draw(),
            Line::None => {}
        }
    }
}

fn plus(painter: &mut Painter<'_>, c: Point, color: masonry::peniko::Color) {
    let h = PLUS / 2.0 - 2.0;
    let mut path = BezPath::new();
    path.move_to((c.x - h, c.y));
    path.line_to((c.x + h, c.y));
    path.move_to((c.x, c.y - h));
    path.line_to((c.x, c.y + h));
    painter.stroke(&path, &Stroke::new(1.5).with_caps(masonry::kurbo::Cap::Round), color).draw();
}

