use std::ops::Range;

use masonry::core::{BrushIndex, StyleSet};
use masonry::parley::{Affinity, Alignment, AlignmentOptions, Cursor, FontContext, FontWeight, IndentOptions, Layout, LayoutContext, OverflowWrap, StyleProperty};

use super::super::embed::{Font, Spec};
use super::super::markdown::{self, Block, Decor, Fonts};

pub fn measure_ch(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, fonts: &Fonts) -> f32 {
    const SAMPLE: &str = "0000000000";
    let mut builder = lcx.ranged_builder(fcx, SAMPLE, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(fonts.mono.clone()));
    builder.push_default(StyleProperty::FontSize(fonts.size));
    let mut layout = builder.build(SAMPLE);
    layout.break_all_lines(None);
    layout.width() / SAMPLE.len() as f32
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
            builder.push_default(StyleProperty::FontFamily(crate::ui::theme::ui_font("")));
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
    if spec.markdown {
        markdown::style(&mut builder, &Block::Text, spec.text, None, fonts);
    }
    for link in spec.links {
        builder.push(StyleProperty::Brush(markdown::LINK), link.clone());
        if spec.underline {
            builder.push(StyleProperty::Underline(true), link.clone());
            builder.push(StyleProperty::UnderlineBrush(Some(markdown::LINK)), link.clone());
        }
    }
    let mut layout = builder.build(spec.text);
    let wrap = spec.wrap.map(|w| w as f32);
    layout.break_all_lines(wrap);
    layout.align(wrap, spec.align, AlignmentOptions::default());
    layout
}

pub fn build(
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    styles: &StyleSet,
    text: &str,
    underline: Option<Range<usize>>,
    width: Option<f32>,
    markdown: Option<(&Fonts, &Block, Option<usize>)>,
) -> (Layout<BrushIndex>, Decor) {
    let mut builder = lcx.ranged_builder(fcx, text, 1.0, true);
    for prop in styles.inner().values() {
        builder.push_default(prop.to_owned());
    }
    if markdown.is_some() {
        builder.push_default(StyleProperty::OverflowWrap(OverflowWrap::Anywhere));
    }
    let decor = match markdown {
        Some((fonts, block, caret)) => markdown::style(&mut builder, block, text, caret, fonts),
        None => Decor::default(),
    };
    if let Some(range) = underline {
        builder.push(StyleProperty::Underline(true), range);
    }
    let mut layout = builder.build(text);
    let wrap = width.map(|w| (w - decor.inset as f32).max(0.0));
    if let Some(hang) = decor.hang.filter(|h| *h > 0 && *h <= text.len()) {
        layout.break_all_lines(None);
        let x = Cursor::from_byte_index(&layout, hang, Affinity::Downstream).geometry(&layout, 0.0).x0;
        layout.set_text_indent(x as f32, IndentOptions { each_line: false, hanging: true });
    }
    layout.break_all_lines(wrap);
    layout.align(wrap, Alignment::Start, AlignmentOptions::default());
    (layout, decor)
}
