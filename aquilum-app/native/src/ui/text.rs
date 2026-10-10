//! Однострочный текст интерфейса: шрифт из настроек, высота строки, многоточие при нехватке
//! места (как `white-space: nowrap; text-overflow: ellipsis`).

use masonry::core::{BrushIndex, PaintCtx};
use masonry::parley::style::StyleProperty;
use masonry::parley::{FontWeight, Layout, LineHeight};

use super::theme;

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

/// Раскладка строки `text` шириной не больше `width`; кэшируется до смены текста, ширины
/// или начертания.
#[derive(Default)]
pub struct Line {
    cached: Option<(Layout<BrushIndex>, f64, String, Option<Style>)>,
}

impl Line {
    /// Забыть раскладку: изменились шрифты.
    pub fn clear(&mut self) {
        self.cached = None;
    }

    pub fn layout(&mut self, ctx: &mut PaintCtx<'_>, text: &str, width: f64, style: Style) -> &Layout<BrushIndex> {
        let fresh = self.cached.as_ref().is_some_and(|(_, w, t, s)| *w == width && t == text && *s == Some(style));
        if !fresh {
            self.cached = Some((build(ctx, text, width, style), width, text.to_owned(), Some(style)));
        }
        &self.cached.as_ref().expect("раскладка построена выше").0
    }
}

/// Ширина строки без ограничения — для подбора ширины кнопок и пунктов меню.
pub fn measure(ctx: &mut masonry::core::MeasureCtx<'_>, text: &str, style: Style) -> f64 {
    let (font_cx, layout_cx) = ctx.text_contexts();
    measure_with(font_cx, layout_cx, text, style)
}

pub fn measure_with(font_cx: &mut masonry::parley::FontContext, layout_cx: &mut masonry::parley::LayoutContext<BrushIndex>, text: &str, style: Style) -> f64 {
    let family = theme::ui_font_settings().family.clone();
    let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(theme::ui_font(&family)));
    builder.push_default(StyleProperty::FontSize(style.size));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight)));
    let mut layout = builder.build(text);
    layout.break_all_lines(None);
    f64::from(layout.width())
}

fn build(ctx: &mut PaintCtx<'_>, text: &str, width: f64, style: Style) -> Layout<BrushIndex> {
    let family = theme::ui_font_settings().family.clone();
    let (font_cx, layout_cx) = ctx.text_contexts();
    let mut make = |text: &str| {
        let mut builder = layout_cx.ranged_builder(font_cx, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(theme::ui_font(&family)));
        builder.push_default(StyleProperty::FontSize(style.size));
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight)));
        builder.push_default(StyleProperty::LineHeight(LineHeight::Absolute(style.line_height as f32)));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    };
    let layout = make(text);
    if f64::from(layout.width()) <= width {
        return layout;
    }
    // Самый длинный префикс, который влезает вместе с многоточием.
    let chars: Vec<char> = text.chars().collect();
    let truncated = |n: usize| chars[..n].iter().collect::<String>().trim_end().to_owned() + "…";
    let (mut lo, mut hi) = (0, chars.len());
    while lo < hi {
        let mid = (lo + hi).div_ceil(2);
        if f64::from(make(&truncated(mid)).width()) <= width { lo = mid } else { hi = mid - 1 }
    }
    make(&truncated(lo))
}

/// Многострочная подпись шрифтом интерфейса: кегль из настроек, свой цвет, вес и высота
/// строки — для заголовков и описаний в диалогах и настройках.
pub fn label(text: &str, color: masonry::peniko::Color, weight: Option<f32>, line_height: f32) -> masonry::core::NewWidget<masonry::widgets::Label> {
    use masonry::core::{NewWidget, PropertySet};
    use masonry::properties::{ContentColor, LineBreaking};

    let font = theme::ui_font_settings();
    let label = masonry::widgets::Label::new(text)
        .with_style(StyleProperty::FontFamily(theme::ui_font(&font.family)))
        .with_style(StyleProperty::FontSize(font.size))
        .with_style(StyleProperty::FontWeight(FontWeight::new(weight.unwrap_or(font.weight))))
        .with_style(StyleProperty::LineHeight(LineHeight::Absolute(line_height)));
    NewWidget::new(label).with_props(PropertySet::new().with(ContentColor::new(color)).with(LineBreaking::WordWrap))
}
