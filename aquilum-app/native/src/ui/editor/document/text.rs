use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{Affinity, Alignment, AlignmentOptions, Cursor, FontContext, FontWeight, IndentOptions, Layout, LayoutContext, OverflowWrap, StyleProperty};

use super::super::embed::{Font, Spec};
use super::super::markdown::{self, Block, Decor, Fonts, Frame};

pub fn measure(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, fonts: &mut Fonts) {
    const SAMPLE: &str = "0000000000";
    let mut mono = |text: &str| {
        let mut builder = lcx.ranged_builder(fcx, text, 1.0, true);
        builder.push_default(StyleProperty::FontFamily(fonts.mono.clone()));
        builder.push_default(StyleProperty::FontSize(fonts.size));
        builder.push_default(StyleProperty::Brush(markdown::MUTED));
        let mut layout = builder.build(text);
        layout.break_all_lines(None);
        layout
    };
    fonts.ch = mono(SAMPLE).width() / SAMPLE.len() as f32;
    fonts.digits = SAMPLE.char_indices().map(|(i, _)| mono(&i.to_string())).collect();
    let quote = spec_layout(fcx, lcx, styles, fonts, &Spec::plain("> >", Font::Editor, markdown::TEXT));
    let x = |at: usize| Cursor::from_byte_index(&quote, at, Affinity::Downstream).geometry(&quote, 0.0).x0 as f32;
    (fonts.mark, fonts.space, fonts.line) = (x(1), x(2) - x(1), quote.height());
}

pub fn spec_layout(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, styles: &StyleSet, fonts: &Fonts, spec: &Spec) -> Layout<BrushIndex> {
    let mut builder = lcx.ranged_builder(fcx, spec.text, 1.0, true);
    match spec.font {
        Font::Editor => {
            for prop in styles.inner().values() {
                builder.push_default(prop.to_owned());
            }
        }
        Font::Ui(px) => {
            let font = crate::ui::theme::ui_font_settings();
            builder.push_default(StyleProperty::FontFamily(crate::ui::theme::ui_font(&font.family)));
            builder.push_default(StyleProperty::FontWeight(FontWeight::new(font.weight)));
            builder.push_default(StyleProperty::FontSize(f32::from(px)));
        }
        Font::Mono(px) => {
            builder.push_default(StyleProperty::FontFamily(fonts.mono.clone()));
            builder.push_default(StyleProperty::FontSize(f32::from(px)));
        }
    }
    builder.push_default(StyleProperty::Brush(spec.brush.clone()));
    builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
    if spec.bold {
        builder.push_default(StyleProperty::FontWeight(FontWeight::BOLD));
    } else if let Some(weight) = spec.weight {
        builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight)));
    }
    for link in spec.links {
        markdown::push(&mut builder, StyleProperty::Brush(markdown::LINK), link.clone());
        if spec.underline {
            markdown::push(&mut builder, StyleProperty::Underline(true), link.clone());
            markdown::push(&mut builder, StyleProperty::UnderlineBrush(Some(markdown::LINK)), link.clone());
        }
    }
    let mut layout = builder.build(spec.text);
    let wrap = spec.wrap.map(|w| w as f32);
    layout.break_all_lines(wrap);
    layout.align(spec.align, AlignmentOptions::default());
    layout
}

pub fn build(
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    styles: &StyleSet,
    text: &str,
    underline: Option<Range<usize>>,
    width: Option<f32>,
    align: Alignment,
    markdown: Option<(&markdown::Context, &Frame, &Block, Option<Range<usize>>)>,
) -> (Layout<BrushIndex>, Decor) {
    let mut builder = lcx.ranged_builder(fcx, text, 1.0, true);
    for prop in styles.inner().values() {
        builder.push_default(prop.to_owned());
    }
    if markdown.is_some() {
        builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
    }
    let decor = match markdown {
        Some((context, frame, block, selection)) => markdown::style(&mut builder, frame, block, text, selection, context, width),
        None => Decor::default(),
    };
    if let Some(range) = underline {
        markdown::push(&mut builder, StyleProperty::Underline(true), range);
    }
    let mut layout = builder.build(text);
    let wrap = decor.wrap(width);
    if let Some(hang) = decor.hang.filter(|h| !decor.nowrap && *h > 0 && *h <= text.len()) {
        layout.break_all_lines(None);
        let x = Cursor::from_byte_index(&layout, hang, Affinity::Downstream).geometry(&layout, 0.0).x0;
        layout.set_text_indent(x as f32, IndentOptions { each_line: false, hanging: true });
    }
    layout.break_all_lines(wrap);
    layout.align(align, AlignmentOptions::default());
    (layout, decor)
}
