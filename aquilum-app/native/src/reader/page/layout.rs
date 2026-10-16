use masonry::core::BrushIndex;
use masonry::kurbo::Size;
use masonry::parley::{Alignment, AlignmentOptions, FontContext, Layout, LayoutContext};

use super::text::{BOLD, TextStyle, glyph_width, wrapped};
use super::{Item, Line, Settings};
use crate::reader::flow::{Align, Block, Role};

const THREADS: usize = 4;

fn heading_scale(level: u8) -> f32 {
    match level {
        1 => 2.0,
        2 => 1.5,
        3 => 1.17,
        5 => 0.83,
        6 => 0.67,
        _ => 1.0,
    }
}

fn alignment(align: Option<Align>, role: Role, justify: bool) -> Alignment {
    match (align, role) {
        (Some(Align::Center), _) | (None, Role::Heading(_)) => Alignment::Center,
        (Some(Align::End), _) => Alignment::End,
        (Some(Align::Start), _) => Alignment::Start,
        (None, Role::Pre) => Alignment::Start,
        _ if justify => Alignment::Justify,
        _ => Alignment::Start,
    }
}

fn spacing(role: Role, size: f32, em: f64) -> (f64, f64) {
    match role {
        Role::Heading(_) => (f64::from(size) * 0.67, f64::from(size) * 0.67),
        Role::Pre | Role::Item | Role::Cell => (em * 0.25, em * 0.25),
        Role::Paragraph => (0.0, 0.0),
    }
}

fn font_size(role: Role, settings: &Settings) -> f32 {
    match role {
        Role::Heading(level) => settings.size * heading_scale(level),
        _ => settings.size,
    }
}

fn image_item(natural: Option<(u32, u32)>, width: f64, height: f64, em: f64) -> Item {
    let size = natural.map_or(Size::ZERO, |(w, h)| {
        let (w, h) = (f64::from(w), f64::from(h));
        let k = (width / w).min(height / h).min(1.0);
        Size::new(w * k, h * k)
    });
    Item::Image { size, before: em * 0.5, after: em * 0.5 }
}

pub fn dash_width(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, settings: &Settings) -> f32 {
    glyph_width(fcx, lcx, "-", crate::ui::theme::editor_font(&settings.family), settings.size, settings.weight)
}

pub fn text_layout(block: &Block, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, settings: &Settings, width: f64, dash: f32) -> Option<(Layout<BrushIndex>, Vec<(usize, char)>, f64)> {
    let Block::Text { text, spans, role, depth, indent, align, .. } = block else { return None };
    let mono = crate::ui::theme::editor_font("iA Writer Mono");
    let x = f64::from(*depth) * f64::from(settings.size) * 1.5;
    let wrap = (width - x).max(1.0) as f32;
    let style = TextStyle {
        font: if *role == Role::Pre { mono.clone() } else { crate::ui::theme::editor_font(&settings.family) },
        mono,
        size: font_size(*role, settings),
        weight: if matches!(role, Role::Heading(_)) { BOLD } else { settings.weight },
        line_height: settings.line_height,
        indent: indent.then_some(settings.size),
        dash,
    };
    let hyphenated = settings.hyphenate && matches!(role, Role::Paragraph | Role::Item | Role::Cell);
    let (mut layout, breaks) = wrapped(fcx, lcx, text, spans, hyphenated, &style, wrap);
    layout.align(alignment(*align, *role, settings.justify), AlignmentOptions::default());
    Some((layout, breaks, x))
}

pub fn layout_blocks(
    blocks: &[Block],
    images: &mut dyn FnMut(&str) -> Option<(u32, u32)>,
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    settings: &Settings,
    width: f64,
    height: f64,
) -> Vec<Item> {
    use rayon::prelude::*;
    let (em, dash) = (f64::from(settings.size), dash_width(fcx, lcx, settings));
    let item = |block: &Block, natural: Option<(u32, u32)>, fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>| match (block, text_layout(block, fcx, lcx, settings, width, dash)) {
        (Block::Text { role, .. }, Some((layout, breaks, x))) => {
            let lines = layout
                .lines()
                .map(|line| {
                    let (metrics, range) = (line.metrics(), line.text_range());
                    Line { top: f64::from(metrics.block_min_coord), bottom: f64::from(metrics.block_max_coord), start: range.start, end: range.end }
                })
                .collect();
            let (before, after) = spacing(*role, font_size(*role, settings), em);
            Item::Text { lines, x, before, after, breaks }
        }
        (Block::Image { .. }, _) => image_item(natural, width, height, em),
        _ => Item::Rule,
    };
    let sizes: Vec<Option<(u32, u32)>> = blocks.iter().map(|block| if let Block::Image { src, .. } = block { images(src) } else { None }).collect();
    let chunk = blocks.len().div_ceil(THREADS).max(1);
    let contexts: Vec<FontContext> = blocks.chunks(chunk).map(|_| fcx.clone()).collect();
    let parts: Vec<Vec<Item>> = blocks
        .par_chunks(chunk)
        .zip(sizes.par_chunks(chunk))
        .zip(contexts)
        .map(|((blocks, sizes), mut fcx)| {
            let mut lcx = LayoutContext::new();
            blocks.iter().zip(sizes).map(|(block, &natural)| item(block, natural, &mut fcx, &mut lcx)).collect()
        })
        .collect();
    parts.into_iter().flatten().collect()
}
