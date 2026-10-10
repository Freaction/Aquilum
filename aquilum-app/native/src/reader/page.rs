use masonry::core::{BrushIndex, StyleProperty};
use masonry::kurbo::{Rect, Size};
use masonry::parley::{Alignment, AlignmentOptions, FontContext, FontStyle, FontWeight, IndentOptions, Layout, LayoutContext, FontFamily, LineHeight};
use masonry::peniko::ImageBrush;

use super::flow::{Align, Block, Role, Span};
use super::hyphen::{self, SHY};

const GAP: f64 = 0.07;
const MAX_BLOCK: f64 = 1440.0;
const BOLD: f32 = 700.0;
pub const TEXT: BrushIndex = BrushIndex(0);
pub const MUTED: BrushIndex = BrushIndex(1);

#[derive(Clone, Debug, PartialEq)]
pub struct Settings {
    pub family: String,
    pub size: f32,
    pub weight: f32,
    pub line_height: f32,
    pub justify: bool,
    pub hyphenate: bool,
    pub max_width_ch: u32,
    pub margin: f64,
    pub scrolled: bool,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Geometry {
    pub columns: usize,
    pub column_width: f64,
    pub gap: f64,
    pub left: f64,
    pub top: f64,
    pub height: f64,
    pub size: f64,
}

pub fn advance(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, settings: &Settings) -> f64 {
    let mut builder = lcx.ranged_builder(fcx, "0", 1.0, true);
    builder.push_default(StyleProperty::FontFamily(crate::ui::theme::editor_font(&settings.family)));
    builder.push_default(StyleProperty::FontSize(settings.size));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(settings.weight)));
    let mut layout = builder.build("0");
    layout.break_all_lines(None);
    let width = f64::from(layout.width());
    if width > 0.0 { width } else { f64::from(settings.size) * 0.5 }
}

pub fn geometry(view: Size, settings: &Settings, max_inline: f64) -> Geometry {
    if settings.scrolled {
        let width = max_inline.min(view.width - 2.0 * GAP / 2.0 * view.width).max(1.0);
        return Geometry { columns: 1, column_width: width, gap: 0.0, left: ((view.width - width) / 2.0).round(), top: 0.0, height: view.height, size: width };
    }
    let half = GAP / 2.0 * view.width;
    let portrait = view.height > view.width;
    let count = if portrait { 1.0 } else { 2.0 };
    let middle = (view.width - 4.0 * half).min(max_inline * count - GAP * view.width).max(1.0);
    let size = middle + 2.0 * half;
    let gap = GAP / (1.0 - GAP) * size;
    let columns = (count as usize).min((size / max_inline).ceil().max(1.0) as usize);
    let column_width = (size / columns as f64 - gap).max(1.0);
    let height = (view.height - 2.0 * settings.margin).clamp(1.0, MAX_BLOCK);
    Geometry { columns, column_width, gap, left: (view.width - size) / 2.0, top: ((view.height - height) / 2.0).round(), height, size }
}

impl Geometry {
    pub fn column_rect(&self, index: usize) -> Rect {
        let x = self.left + self.gap / 2.0 + index as f64 * (self.column_width + self.gap);
        Rect::new(x, self.top, x + self.column_width, self.top + self.height)
    }
}

pub enum Item {
    Text { layout: Layout<BrushIndex>, x: f64, before: f64, after: f64, breaks: Vec<(usize, char)> },
    Image { image: Option<ImageBrush>, size: Size, before: f64, after: f64 },
    Rule,
}

impl Item {
    fn margins(&self) -> (f64, f64) {
        match self {
            Item::Text { before, after, .. } | Item::Image { before, after, .. } => (*before, *after),
            Item::Rule => (8.0, 8.0),
        }
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Piece {
    pub item: usize,
    pub from: f64,
    pub to: f64,
    pub y: f64,
}

#[derive(Default, Debug)]
pub struct Column {
    pub pieces: Vec<Piece>,
}

pub struct Laid {
    pub items: Vec<Item>,
    pub columns: Vec<Column>,
    pub height: f64,
}

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

pub fn layout_blocks(
    blocks: &[Block],
    images: &mut dyn FnMut(&str) -> Option<ImageBrush>,
    fcx: &mut FontContext,
    lcx: &mut LayoutContext<BrushIndex>,
    settings: &Settings,
    width: f64,
    height: f64,
) -> Vec<Item> {
    let family = crate::ui::theme::editor_font(&settings.family);
    let mono = crate::ui::theme::editor_font("iA Writer Mono");
    let em = f64::from(settings.size);
    blocks
        .iter()
        .map(|block| match block {
            Block::Text { text, spans, role, depth, indent, align, .. } => {
                let scale = match role {
                    Role::Heading(level) => heading_scale(*level),
                    _ => 1.0,
                };
                let size = settings.size * scale;
                let x = f64::from(*depth) * em * 1.5;
                let wrap = (width - x).max(1.0) as f32;
                let alignment = match (align, role) {
                    (Some(Align::Center), _) | (None, Role::Heading(_)) => Alignment::Center,
                    (Some(Align::End), _) => Alignment::End,
                    (Some(Align::Start), _) => Alignment::Start,
                    (None, Role::Pre) => Alignment::Start,
                    _ if settings.justify => Alignment::Justify,
                    _ => Alignment::Start,
                };
                let style = TextStyle {
                    font: if *role == Role::Pre { mono.clone() } else { family.clone() },
                    mono: mono.clone(),
                    size,
                    weight: if matches!(role, Role::Heading(_)) { BOLD } else { settings.weight },
                    line_height: settings.line_height,
                    indent: indent.then_some(settings.size),
                };
                let hyphenated = settings.hyphenate && matches!(role, Role::Paragraph | Role::Item | Role::Cell);
                let mut breaks = if hyphenated { hyphen::breaks(text) } else { Vec::new() };
                let mut layout = build_text(fcx, lcx, text, spans, &breaks, &style);
                if breaks.is_empty() {
                    layout.break_all_lines(Some(wrap));
                } else {
                    let dash = dash_width(fcx, lcx, &style);
                    let shown = hyphen::apply(text, &breaks);
                    let mut narrow: Vec<bool> = Vec::new();
                    let ends = loop {
                        break_with(&mut layout, |i| if narrow.get(i).copied().unwrap_or(false) { wrap - dash } else { wrap });
                        let ends: Vec<bool> = layout.lines().map(|l| shown[..l.text_range().end].ends_with(SHY)).collect();
                        let mut grew = false;
                        for (i, &end) in ends.iter().enumerate() {
                            if end && !narrow.get(i).copied().unwrap_or(false) {
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
                        layout = build_text(fcx, lcx, text, spans, &breaks, &style);
                        break_with(&mut layout, |i| if !ends.get(i).copied().unwrap_or(false) && narrow.get(i).copied().unwrap_or(false) { wrap - dash } else { wrap });
                    }
                }
                layout.align(Some(wrap), alignment, AlignmentOptions::default());
                let (before, after) = match role {
                    Role::Heading(_) => (f64::from(size) * 0.67, f64::from(size) * 0.67),
                    Role::Pre | Role::Item | Role::Cell => (em * 0.25, em * 0.25),
                    Role::Paragraph => (0.0, 0.0),
                };
                Item::Text { layout, x, before, after, breaks }
            }
            Block::Image { src, .. } => {
                let image = images(src);
                let size = image.as_ref().map_or(Size::ZERO, |i| {
                    let (w, h) = (f64::from(i.image.width), f64::from(i.image.height));
                    let k = (width / w).min(height / h).min(1.0);
                    Size::new(w * k, h * k)
                });
                Item::Image { image, size, before: em * 0.5, after: em * 0.5 }
            }
            Block::Rule => Item::Rule,
        })
        .collect()
}

fn break_with(layout: &mut Layout<BrushIndex>, width: impl Fn(usize) -> f32) {
    let mut breaker = layout.break_lines();
    let mut line = 0;
    while breaker.break_next(width(line).max(1.0)).is_some() {
        line += 1;
    }
    breaker.finish();
}

fn dash_width(fcx: &mut FontContext, lcx: &mut LayoutContext<BrushIndex>, style: &TextStyle) -> f32 {
    let mut builder = lcx.ranged_builder(fcx, "-", 1.0, true);
    builder.push_default(StyleProperty::FontFamily(style.font.clone()));
    builder.push_default(StyleProperty::FontSize(style.size));
    builder.push_default(StyleProperty::FontWeight(FontWeight::new(style.weight)));
    let mut layout = builder.build("-");
    layout.break_all_lines(None);
    layout.width()
}

struct TextStyle {
    font: FontFamily<'static>,
    mono: FontFamily<'static>,
    size: f32,
    weight: f32,
    line_height: f32,
    indent: Option<f32>,
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

pub fn paginate(items: Vec<Item>, height: f64) -> Laid {
    let mut columns = vec![Column::default()];
    let mut y = 0.0;
    let mut total = 0.0;
    for (index, item) in items.iter().enumerate() {
        let (before, after) = item.margins();
        let start = |columns: &mut Vec<Column>, y: &mut f64| {
            columns.push(Column::default());
            *y = 0.0;
        };
        if y > 0.0 {
            y += before;
        }
        total += before;
        match item {
            Item::Text { layout, .. } => {
                for line in layout.lines() {
                    let m = line.metrics();
                    let (top, bottom) = (f64::from(m.min_coord), f64::from(m.max_coord));
                    let tall = bottom - top;
                    if y + tall > height && y > 0.0 {
                        start(&mut columns, &mut y);
                    }
                    let column = columns.last_mut().expect("колонка");
                    match column.pieces.last_mut() {
                        Some(piece) if piece.item == index && (piece.to - top).abs() < 0.5 => piece.to = bottom,
                        _ => column.pieces.push(Piece { item: index, from: top, to: bottom, y }),
                    }
                    y += tall;
                    total += tall;
                }
            }
            Item::Image { size, .. } => {
                let tall = size.height.min(height);
                if y + tall > height && y > 0.0 {
                    start(&mut columns, &mut y);
                }
                columns.last_mut().expect("колонка").pieces.push(Piece { item: index, from: 0.0, to: tall, y });
                y += tall;
                total += tall;
            }
            Item::Rule => {
                if y + 1.0 > height && y > 0.0 {
                    start(&mut columns, &mut y);
                }
                columns.last_mut().expect("колонка").pieces.push(Piece { item: index, from: 0.0, to: 1.0, y });
                y += 1.0;
                total += 1.0;
            }
        }
        y += after;
        total += after;
    }
    if columns.len() > 1 && columns.last().is_some_and(|c| c.pieces.is_empty()) {
        columns.pop();
    }
    Laid { items, columns, height: total }
}
