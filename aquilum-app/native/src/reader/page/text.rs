use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontContext, FontFamily, FontStyle, FontWeight, IndentOptions, Layout, LayoutContext, LineHeight};

use crate::reader::flow::Span;
use crate::reader::hyphen::{self, SHY};

pub(super) const BOLD: f32 = 700.0;
const TEXT: BrushIndex = BrushIndex(0);
const MUTED: BrushIndex = BrushIndex(1);

pub(super) struct TextStyle {
    pub font: FontFamily<'static>,
    pub mono: FontFamily<'static>,
    pub size: f32,
    pub weight: f32,
    pub line_height: f32,
    pub indent: Option<f32>,
}

pub(super) fn glyph_width(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, glyph: &str, font: FontFamily<'static>, size: f32, weight: f32) -> f32 {
    let mut builder = lcx.ranged_builder(fcx, glyph, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(font));
    builder.push_default(StyleProperty::FontSize(size));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(weight)));
    let mut layout = builder.build(glyph);
    layout.break_all_lines(None);
    layout.width()
}

fn break_with(layout: &mut Layout<BrushIndex>, width: impl Fn(usize) -> f32) {
    let mut breaker = layout.break_lines();
    let mut line = 0;
    while breaker.break_next(width(line).max(1.0)).is_some() {
        line += 1;
    }
    breaker.finish();
}

fn flag(flags: &[bool], index: usize) -> bool {
    flags.get(index).copied().unwrap_or(false)
}

fn build_text(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, text: &str, spans: &[Span], breaks: &[(usize, char)], style: &TextStyle) -> Layout<BrushIndex> {
    let shown = hyphen::apply(text, breaks);
    let mut builder = lcx.ranged_builder(fcx, &shown, 1.0, true);
    builder.push_default(StyleProperty::FontFamily(style.font.clone()));
    builder.push_default(StyleProperty::FontSize(style.size));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight)));
    builder.push_default(StyleProperty::LineHeight(LineHeight::FontSizeRelative(style.line_height)));
    builder.push_default(StyleProperty::Brush(TEXT));
    for span in spans {
        let s = span.style;
        let range = hyphen::to_layout(breaks, span.range.start)..hyphen::to_layout(breaks, span.range.end);
        if s.bold {
            builder.push(StyleProperty::FontWeight(FontWeight::new(BOLD)), range.clone());
        }
        if s.italic {
            builder.push(StyleProperty::FontStyle(FontStyle::Italic), range.clone());
        }
        if s.mono {
            builder.push(StyleProperty::FontFamily(style.mono.clone()), range.clone());
        }
        if s.link {
            builder.push(StyleProperty::Brush(MUTED), range.clone());
        }
        if s.small {
            builder.push(StyleProperty::FontSize(style.size * 0.75), range);
        }
    }
    let mut layout = builder.build(&shown);
    if let Some(indent) = style.indent {
        layout.set_text_indent(indent, IndentOptions { each_line: false, hanging: false });
    }
    layout
}

pub(super) fn wrapped(
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    text: &str,
    spans: &[Span],
    hyphenated: bool,
    style: &TextStyle,
    wrap: f32,
) -> (Layout<BrushIndex>, Vec<(usize, char)>) {
    let mut breaks = if hyphenated { hyphen::breaks(text) } else { Vec::new() };
    let mut layout = build_text(fcx, lcx, text, spans, &breaks, style);
    if breaks.is_empty() {
        layout.break_all_lines(Some(wrap));
        return (layout, breaks);
    }
    let dash = glyph_width(fcx, lcx, "-", style.font.clone(), style.size, style.weight);
    let shown = hyphen::apply(text, &breaks);
    let mut narrow: Vec<bool> = Vec::new();
    let ends = loop {
        break_with(&mut layout, |i| if flag(&narrow, i) { wrap - dash } else { wrap });
        let ends: Vec<bool> = layout.lines().map(|l| shown[..l.text_range().end].ends_with(SHY)).collect();
        let mut grew = false;
        for (i, &end) in ends.iter().enumerate() {
            if end && !flag(&narrow, i) {
                if narrow.len() <= i {
                    narrow.resize(i + 1, false);
                }
                narrow[i] = true;
                grew = true;
            }
        }
        if !grew {
            break ends;
        }
    };
    let positions: Vec<usize> = layout.lines().map(|l| l.text_range().end).collect();
    let marks: Vec<bool> = breaks.iter().map(|b| positions.iter().zip(&ends).any(|(&p, &e)| e && p == hyphen::to_layout(&breaks, b.0))).collect();
    if marks.iter().any(|&m| m) {
        for (entry, mark) in breaks.iter_mut().zip(marks) {
            if mark {
                entry.1 = '-';
            }
        }
        layout = build_text(fcx, lcx, text, spans, &breaks, style);
        break_with(&mut layout, |i| if !flag(&ends, i) && flag(&narrow, i) { wrap - dash } else { wrap });
    }
    (layout, breaks)
}
