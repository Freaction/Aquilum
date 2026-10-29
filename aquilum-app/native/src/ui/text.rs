//! Однострочный текст интерфейса: шрифт из настроек, высота строки, многоточие при нехватке
//! места (как `white-space: nowrap; text-overflow: ellipsis`).

use masonry::core::{BrushIndex, PaintCtx};
use masonry::imaging::Painter;
use masonry::kurbo::{Point, Rect, RoundedRect};
use masonry::parley::style::StyleProperty;
use masonry::parley::{Affinity, Cursor, FontContext, FontFamily, FontWeight, Layout, LayoutContext, LineHeight, RangedBuilder, Selection};

use super::theme;
use super::tokens::size;

/// Начертание строки: кегль и вес (по умолчанию — из настроек шрифта интерфейса), высота строки.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Style {
    pub size: f32,
    pub weight: f32,
    pub line_height: f64,
}

impl Style {
    /// Шрифт интерфейса из настроек с заданной высотой строки.
    pub fn ui(line_height: f64) -> Self {
        let font = theme::ui_font_settings();
        Style { size: font.size, weight: font.weight, line_height }
    }
}

#[derive(Default)]
pub struct Line {
    full: Option<(Layout<BrushIndex>, String, Style)>,
    ellipsis: Option<f64>,
    cut: Option<(Layout<BrushIndex>, usize)>,
}

impl Line {
    pub fn clear(&mut self) {
        *self = Line::default();
    }

    pub fn width(&mut self, font_cx: &mut FontContext, layout_cx: &mut LayoutContext<BrushIndex>, text: &str, style: Style) -> f64 {
        if self.full.as_ref().is_none_or(|(_, t, s)| t != text || *s != style) {
            *self = Line { full: Some((shape(font_cx, layout_cx, text, style), text.to_owned(), style)), ..Line::default() };
        }
        f64::from(self.full.as_ref().expect("раскладка построена выше").0.width())
    }

    pub fn layout(&mut self, ctx: &mut PaintCtx<'_>, text: &str, width: f64, style: Style) -> &Layout<BrushIndex> {
        let (font_cx, layout_cx) = ctx.text_contexts();
        let fits = self.width(font_cx, layout_cx, text, style) <= width;
        let full = &self.full.as_ref().expect("раскладка построена выше").0;
        if fits {
            return full;
        }
        let ellipsis = *self.ellipsis.get_or_insert_with(|| f64::from(shape(font_cx, layout_cx, "…", style).width()));
        let end = cut(full, width - ellipsis).unwrap_or(text.len());
        if self.cut.as_ref().is_none_or(|(_, at)| *at != end) {
            let shown = text[..end].trim_end().to_owned() + "…";
            self.cut = Some((shape(font_cx, layout_cx, &shown, style), end));
        }
        &self.cut.as_ref().expect("обрезка построена выше").0
    }
}

fn cut(layout: &Layout<BrushIndex>, room: f64) -> Option<usize> {
    let mut x = 0.0;
    for cluster in layout.lines().flat_map(|line| line.runs()).flat_map(|run| run.clusters()) {
        x += f64::from(cluster.advance());
        if x > room {
            return Some(cluster.text_range().start);
        }
    }
    None
}

pub fn builder<'a>(
    font_cx: &'a mut FontContext,
    layout_cx: &'a mut LayoutContext<BrushIndex>,
    text: &'a str,
    family: FontFamily<'_>,
    style: Style,
) -> RangedBuilder<'a, BrushIndex> {
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(family));
    builder.push_default(StyleProperty::FontSize(style.size));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight)));
    builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(style.line_height as f32)));
    builder
}

fn shape(font_cx: &mut FontContext, layout_cx: &mut LayoutContext<BrushIndex>, text: &str, style: Style) -> Layout<BrushIndex> {
    let family = theme::ui_font_settings().family.clone();
    let mut layout = builder(font_cx, layout_cx, text, theme::ui_font(&family), style).build(text);
    layout.break_all_lines(None);
    layout
}

fn plain(text: &str, paint: super::widgets::Paint, weight: Option<f32>, leading: super::widgets::Leading) -> super::widgets::Label {
    let label = super::widgets::Label::new(text, paint).leading(leading);
    match weight {
        Some(weight) => label.weight(weight),
        None => label,
    }
}

pub fn label(text: &str, paint: super::widgets::Paint, weight: Option<f32>, leading: super::widgets::Leading) -> masonry::core::NewWidget<super::widgets::Label> {
    masonry::core::NewWidget::new(plain(text, paint, weight, leading))
}

pub fn selectable(text: &str, paint: super::widgets::Paint, weight: Option<f32>, leading: super::widgets::Leading) -> masonry::core::NewWidget<super::widgets::Label> {
    masonry::core::NewWidget::new(plain(text, paint, weight, leading).selectable())
}

pub fn paint_marks(painter: &mut Painter<'_>, layout: &Layout<BrushIndex>, marks: &[(usize, usize)], origin: Point) {
    let color = theme::current().search_match_bg;
    for &(start, end) in marks {
        let selection = Selection::new(Cursor::from_byte_index(layout, start, Affinity::Downstream), Cursor::from_byte_index(layout, end, Affinity::Upstream));
        for (rect, _) in selection.geometry(layout) {
            let rect = Rect::new(rect.x0, rect.y0, rect.x1, rect.y1) + origin.to_vec2();
            painter.fill(RoundedRect::from_rect(rect, size::ROUNDED_SM), color).draw();
        }
    }
}
