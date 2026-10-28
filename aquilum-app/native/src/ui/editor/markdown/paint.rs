use masonry::core::BrushIndex;
use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{BezPath, Cap, Circle, Join, Point, Rect, RoundedRect, Stroke, Vec2};
use masonry::parley::{Affinity, Cursor, Layout, LineMetrics, PositionedLayoutItem, Selection};
use masonry::peniko::{Brush, Color};

use super::super::embed::media::draw_image;
use super::metrics::{BULLET, CHECKBOX_RADIUS, INLINE_CODE_PADDING, TAG_PADDING};
use super::{Boxed, Decor};
use crate::ui::{palette, theme};

const TAG_PAD_X: f64 = TAG_PADDING as f64;
const TAG_HEIGHT: f64 = 24.0;
const TAG_ALPHA: f32 = 0.1;
const TAG_COLOR_ALPHA: f32 = 0.14;
const CODE_PAD_X: f64 = INLINE_CODE_PADDING as f64;
const CODE_PAD_Y: f64 = 2.0;
const CODE_RADIUS: f64 = 2.0;
const MARK_PAD_X: f64 = 1.0;
const MARK_ALPHA: f32 = 0.2;

pub fn range_rects(layout: &Layout<BrushIndex>, from: usize, to: usize) -> Vec<(Rect, usize)> {
    let selection = Selection::new(Cursor::from_byte_index(layout, from, Affinity::Downstream), Cursor::from_byte_index(layout, to, Affinity::Upstream));
    selection.geometry(layout).into_iter().map(|(b, line)| (Rect::new(b.x0, b.y0, b.x1, b.y1), line)).collect()
}

pub fn x_at(layout: &Layout<BrushIndex>, offset: usize) -> f64 {
    Cursor::from_byte_index(layout, offset, Affinity::Downstream).geometry(layout, 0.0).x0
}

pub fn line_at(layout: &Layout<BrushIndex>, y: f64) -> Option<LineMetrics> {
    let y = y as f32;
    layout.lines().map(|l| *l.metrics()).find(|m| m.block_min_coord <= y && y < m.block_max_coord)
}

pub fn line_middle(layout: &Layout<BrushIndex>) -> f64 {
    layout.lines().next().map_or(0.0, |l| f64::from(l.metrics().block_min_coord + l.metrics().block_max_coord) / 2.0)
}

fn fill_range(painter: &mut Painter<'_, impl PaintSink + ?Sized>, layout: &Layout<BrushIndex>, range: &std::ops::Range<usize>, origin: Vec2, pad_x: f64, color: Color) {
    for (bounds, _) in range_rects(layout, range.start, range.end) {
        let Some(m) = line_at(layout, (bounds.y0 + bounds.y1) / 2.0) else { continue };
        let (top, bottom) = (f64::from(m.content_block_min_coord) - CODE_PAD_Y, f64::from(m.content_block_max_coord) + CODE_PAD_Y);
        let shape = RoundedRect::new(bounds.x0 - pad_x, top, bounds.x1 + pad_x, bottom, CODE_RADIUS) + origin;
        painter.fill(shape, color).draw();
    }
}

pub fn tag_chips(layout: &Layout<BrushIndex>, decor: &Decor, origin: Vec2) -> Vec<(RoundedRect, Option<usize>)> {
    decor
        .tags
        .iter()
        .map(|(tag, slot)| {
            let start = Cursor::from_byte_index(layout, tag.start, Affinity::Downstream).geometry(layout, 0.0);
            let middle = origin.y + (start.y0 + start.y1) / 2.0;
            (RoundedRect::new(origin.x + start.x0 - TAG_PAD_X, middle - TAG_HEIGHT / 2.0, origin.x + x_at(layout, tag.end) + TAG_PAD_X, middle + TAG_HEIGHT / 2.0, TAG_HEIGHT / 2.0), *slot)
        })
        .collect()
}

pub fn paint_bullet(painter: &mut Painter<'_, impl PaintSink + ?Sized>, center: Point) {
    painter.fill(Circle::new(center, BULLET / 2.0), theme::current().editor_list_marker_color).draw();
}

pub fn paint_check(painter: &mut Painter<'_, impl PaintSink + ?Sized>, bounds: Rect, checked: bool) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, CHECKBOX_RADIUS);
    if checked {
        painter.fill(shape, tm.bg_accent).draw();
        let c = bounds.center();
        let mut check = BezPath::new();
        check.move_to((c.x - 4.5, c.y + 0.5));
        check.line_to((c.x - 1.5, c.y + 3.5));
        check.line_to((c.x + 4.5, c.y - 3.0));
        painter.stroke(&check, &Stroke::new(2.0).with_caps(Cap::Round).with_join(Join::Round), tm.icon_on_accent).draw();
    } else {
        painter.fill(shape, tm.bg_surface_overlay).draw();
    }
}

pub fn paint_backgrounds(painter: &mut Painter<'_, impl PaintSink + ?Sized>, layout: &Layout<BrushIndex>, decor: &Decor, origin: Vec2) {
    let tm = theme::current();
    for (shape, slot) in tag_chips(layout, decor, origin) {
        let color = match slot.and_then(|slot| palette::color(palette::NAMES[slot], &tm)) {
            Some(color) => color.multiply_alpha(TAG_COLOR_ALPHA),
            None => tm.bg_accent.multiply_alpha(TAG_ALPHA),
        };
        painter.fill(shape, color).draw();
    }
    for mark in &decor.marks {
        fill_range(painter, layout, mark, origin, MARK_PAD_X, tm.list_callout_highlight.multiply_alpha(MARK_ALPHA));
    }
    for code in &decor.codes {
        fill_range(painter, layout, code, origin, CODE_PAD_X, tm.editor_code_bg);
    }
}

pub fn paint_boxes(painter: &mut Painter<'_, impl PaintSink + ?Sized>, layout: &Layout<BrushIndex>, decor: &Decor, origin: Vec2, palette: &[Brush]) {
    if decor.boxes.is_empty() {
        return;
    }
    for item in layout.lines().flat_map(|line| line.items()) {
        if let PositionedLayoutItem::InlineBox(b) = item
            && let Some(boxed) = decor.boxes.get(b.id as usize)
        {
            let at = origin + Vec2::new(f64::from(b.x), f64::from(b.y));
            match boxed {
                Boxed::Formula(formula) => formula.paint(painter, at, palette),
                Boxed::Image { image, crop } => draw_image(painter, image, Rect::from_origin_size(at.to_point(), (f64::from(b.width), f64::from(b.height))), *crop),
            }
        }
    }
}
