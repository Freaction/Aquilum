use masonry::core::{BrushIndex, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, BezPath, Point, Rect, RoundedRect, Stroke, Vec2};
use masonry::parley::Layout;
use masonry::peniko::Brush;

use super::{ADD, Hit, Line, Overlay, PAD_Y, PLUS, STRIP, TOP, TableView};
use crate::ui::editor::markdown;
use crate::ui::scroll::bar::{Scrollbar, Span};
use crate::ui::theme;
use crate::ui::tokens::size;

impl TableView {
    pub fn paint(&self, painter: &mut Painter<'_>, x: f64, y: f64, palette: &[Brush], overlay: &Overlay) {
        let tm = theme::current();
        let s = overlay.scroll_x;
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
            let mut seen = 0;
            for cell in &self.cells {
                let r = Rect::new(tx + cell.rect.x0, TOP + cell.rect.y0, tx + cell.rect.x1, TOP + cell.rect.y1);
                let px = Rect::new(x + r.x0.round() + 0.5, y + r.y0.round() + 0.5, x + r.x1.round() + 0.5, y + r.y1.round() + 0.5);
                painter.stroke(px, &stroke, tm.editor_table_border).draw();
                let o = Vec2::new(x, y) + self.cell_origin(cell, s).to_vec2();
                markdown::paint_backgrounds(painter, &cell.layout, &cell.decor, o);
                if let Some(search) = &overlay.search {
                    for found in search.ranges(&cell.text, &mut seen) {
                        for (rect, _) in markdown::range_rects(&cell.layout, found.start, found.end) {
                            painter.fill(rect + o, tm.search_match_bg).draw();
                        }
                    }
                }
                render_text(painter, Affine::translate(o), &cell.layout, palette, true);
                markdown::paint_boxes(painter, &cell.layout, &cell.decor, o, palette);
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
            let row = Rect::new(x, y + bottom, x + self.edge(), y + bottom + ADD);
            let col = Rect::new(x + self.edge(), y + PAD_Y, x + self.edge() + ADD, y + bottom);
            for band in [row, col] {
                painter.fill(band, tm.editor_table_add_bg).draw();
                painter.stroke(band.inset(-0.5), &stroke, tm.editor_table_add_border).draw();
                plus(painter, band.center(), tm.icon_faint);
            }
        }
        if let Some(span) = self.bar_span(s) {
            let mut bar = Scrollbar::new(Axis::Horizontal);
            bar.set_hovered(overlay.bar_active);
            bar.paint(painter, &Span { track: span.track + Vec2::new(x, y), ..span });
        }
        let (valid, invalid) = (tm.bg_accent, tm.text_muted.multiply_alpha(0.6));
        match overlay.line {
            Line::Row(ly, ok) => painter.fill(RoundedRect::new(x, y + ly - 1.0, x + self.edge(), y + ly + 1.0, 1.0), if ok { valid } else { invalid }).draw(),
            Line::Col(lx, ok) if lx >= self.corner && lx <= self.scroll_w => painter.fill(RoundedRect::new(x + lx - 1.0, y + PAD_Y, x + lx + 1.0, y + bottom, 1.0), if ok { valid } else { invalid }).draw(),
            Line::Col(..) | Line::None => {}
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
