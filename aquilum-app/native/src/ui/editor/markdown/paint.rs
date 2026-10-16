use masonry::core::BrushIndex;
use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{Rect, RoundedRect, Vec2};
use masonry::parley::{Affinity, Cursor, Layout, PositionedLayoutItem, Selection};
use masonry::peniko::{Brush, Color};

use super::Decor;
use super::metrics::{INLINE_CODE_PADDING, TAG_PADDING};
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

fn fill_range(painter: &mut Painter<'_, impl PaintSink + ?Sized>, layout: &Layout<BrushIndex>, range: &std::ops::Range<usize>, origin: Vec2, pad_x: f64, color: Color) {
    for (bounds, _) in range_rects(layout, range.start, range.end) {
        let middle = ((bounds.y0 + bounds.y1) / 2.0) as f32;
        let Some(m) = layout.lines().map(|l| *l.metrics()).find(|m| m.block_min_coord <= middle && middle < m.block_max_coord) else { continue };
        let (top, bottom) = (f64::from(m.content_block_min_coord) - CODE_PAD_Y, f64::from(m.content_block_max_coord) + CODE_PAD_Y);
        let shape = RoundedRect::new(bounds.x0 - pad_x, top, bounds.x1 + pad_x, bottom, CODE_RADIUS) + origin;
        painter.fill(shape, color).draw();
    }
}

pub fn tag_chips(layout: &Layout<BrushIndex>, decor: &Decor, origin: Vec2) -> Vec<(RoundedRect, Option<usize>)> {
    let caret = |offset: usize| Cursor::from_byte_index(layout, offset, Affinity::Downstream).geometry(layout, 0.0);
    decor
        .tags
        .iter()
        .map(|(tag, slot)| {
            let (start, end) = (caret(tag.start), caret(tag.end));
            let middle = origin.y + (start.y0 + start.y1) / 2.0;
            (RoundedRect::new(origin.x + start.x0 - TAG_PAD_X, middle - TAG_HEIGHT / 2.0, origin.x + end.x0 + TAG_PAD_X, middle + TAG_HEIGHT / 2.0, TAG_HEIGHT / 2.0), *slot)
        })
        .collect()
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

pub fn paint_formulas(painter: &mut Painter<'_, impl PaintSink + ?Sized>, layout: &Layout<BrushIndex>, decor: &Decor, origin: Vec2, palette: &[Brush]) {
    if decor.formulas.is_empty() {
        return;
    }
    for item in layout.lines().flat_map(|line| line.items()) {
        if let PositionedLayoutItem::InlineBox(b) = item
            && let Some(formula) = decor.formulas.get(b.id as usize)
        {
            formula.paint(painter, origin + Vec2::new(f64::from(b.x), f64::from(b.y)), palette);
        }
    }
}
