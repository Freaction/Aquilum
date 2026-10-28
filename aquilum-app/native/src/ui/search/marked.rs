use masonry::core::{BrushIndex, PaintCtx, render_text};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Point, Rect, RoundedRect};
use masonry::parley::style::StyleProperty;
use masonry::parley::{Affinity, Cursor, Layout, Selection};
use masonry::peniko::Color;

use super::card::{CardData, Marks};
use crate::ui::text::{Style, builder};
use crate::ui::theme;
use crate::ui::tokens::{number, size};

pub(super) fn marked_layout(ctx: &mut PaintCtx<'_>, text: &str, marks: &[(usize, usize)], line_height: f32, width: Option<f32>) -> Layout<BrushIndex> {
    let font = theme::ui_font_settings();
    let (font_cx, layout_cx) = ctx.text_contexts();
    let style = Style { size: size::FONT_SIZE_UI_BASE as f32, weight: font.weight, line_height: f64::from(line_height) };
    let mut builder = builder(font_cx, layout_cx, text, theme::ui_font(&font.family), style);
    for &(start, end) in marks {
        builder.push(StyleProperty::Brush(BrushIndex(1)), start..end);
    }
    let mut layout = builder.build(text);
    layout.break_all_lines(width);
    layout
}

pub(super) fn paint_marked(painter: &mut Painter<'_>, layout: &Layout<BrushIndex>, marks: &[(usize, usize)], origin: Point, color: Color) {
    let t = theme::current();
    for &(start, end) in marks {
        let selection = Selection::new(
            Cursor::from_byte_index(layout, start, Affinity::Downstream),
            Cursor::from_byte_index(layout, end, Affinity::Upstream),
        );
        for (rect, _) in selection.geometry(layout) {
            let rect = Rect::new(rect.x0, rect.y0, rect.x1, rect.y1) + origin.to_vec2();
            painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_SM), t.search_match_bg).draw();
        }
    }
    render_text(painter, Affine::translate(origin.to_vec2()), layout, &[color.into(), t.text_primary.into()], true);
}

pub(super) const LINE: f32 = size::SIZE_20 as f32;

pub(super) fn snippet_line() -> f32 {
    (size::FONT_SIZE_UI_BASE * number::FONT_LINE_HEIGHT_SNUG) as f32
}

pub(super) struct CardLayouts {
    key: (f32, String, f32),
    pub count: Layout<BrushIndex>,
    pub title: Layout<BrushIndex>,
    pub extension: Option<(Layout<BrushIndex>, Marks)>,
    pub snippet: Layout<BrushIndex>,
}

fn key(width: f32) -> (f32, String, f32) {
    let font = theme::ui_font_settings();
    (width, font.family.clone(), font.weight)
}

impl CardLayouts {
    pub fn fresh(&self, width: f32) -> bool {
        let font = theme::ui_font_settings();
        self.key.0 == width && self.key.1 == font.family && self.key.2 == font.weight
    }

    pub fn build(ctx: &mut PaintCtx<'_>, data: &CardData, width: f32) -> Self {
        let extension = (!data.extension.is_empty()).then(|| {
            let marks: Marks = data.extension_marks.iter().map(|(s, e)| (s + 1, e + 1)).collect();
            (marked_layout(ctx, &format!(".{}", data.extension), &marks, LINE, None), marks)
        });
        Self {
            key: key(width),
            count: marked_layout(ctx, &data.count, &[], LINE, None),
            title: marked_layout(ctx, &data.title, &data.title_marks, LINE, None),
            extension,
            snippet: marked_layout(ctx, &data.snippet, &data.snippet_marks, snippet_line(), Some(width)),
        }
    }
}
