mod builder;
mod callout;
mod code;
mod heading;
mod item;
mod spans;

use std::collections::HashSet;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontStyle, RangedBuilder};

use super::super::embed::panel::{KINDS, PAD};
use super::super::resolve::Resolver;
use super::blocks::{Block, Callout, Frame};
use super::brushes::{HIDDEN, MUTED, SYNTAX};
use super::metrics::{Fonts, LIST_LEVEL, TAB_SIZE};
use super::Decor;
use builder::hide;
use callout::style_callout;
use code::style_code;
use heading::style_heading;
use item::style_item;
use spans::style_spans;

pub use builder::push;

pub struct Context<'a> {
    pub fonts: &'a Fonts,
    pub unresolved: &'a HashSet<String>,
    pub resolver: Option<&'a dyn Resolver>,
    pub inside: bool,
    pub gutter: usize,
}

fn style_frame(builder: &mut RangedBuilder<'_, BrushIndex>, frame: &Frame, text: &str, shown: bool, fonts: &Fonts, decor: &mut Decor) {
    let cut = frame.cut(text);
    hide(builder, 0..cut, 0.0);
    decor.x = frame.origin(text, fonts);
    decor.pad.1 = f64::from(frame.closes) * PAD;
    if frame.quote > frame.panel {
        push(builder, StyleProperty::Brush(if shown { SYNTAX } else { HIDDEN }), cut..frame.marker);
        decor.hang = Some(frame.lead);
    }
    hide(builder, frame.marker..frame.lead, 0.0);
}

pub fn style(builder: &mut RangedBuilder<'_, BrushIndex>, frame: &Frame, block: &Block, text: &str, caret: Option<usize>, context: &Context, width: Option<f32>) -> Decor {
    let fonts = context.fonts;
    let mut decor = Decor { caret, ..Decor::default() };
    for (i, _) in text.match_indices('\t') {
        hide(builder, i..i + 1, fonts.ch * TAB_SIZE);
    }
    style_frame(builder, frame, text, caret.is_some(), fonts, &mut decor);
    let indent = text.len() - text.trim_start().len();
    let content = block.content(frame).max(indent).min(text.len());
    let empty = text[content..].trim().is_empty();
    match block {
        Block::Text | Block::Table { .. } => {}
        Block::Image => return decor,
        Block::Comment => {
            push(builder, StyleProperty::Brush(MUTED), frame.lead..text.len());
            push(builder, StyleProperty::FontStyle(FontStyle::Italic), frame.lead..text.len());
            return decor;
        }
        Block::Underline | Block::Rule => {
            push(builder, StyleProperty::Brush(if caret.is_some() || matches!(block, Block::Underline) { SYNTAX } else { HIDDEN }), frame.lead..text.len());
            return decor;
        }
        Block::Definition { marker } => {
            decor.x += LIST_LEVEL;
            decor.hang = Some(frame.lead + marker);
            push(builder, StyleProperty::Brush(SYNTAX), frame.lead..frame.lead + marker);
        }
        Block::Meta => {
            let body = &text[indent..];
            let key = if body.starts_with("---") { body.len() } else if body.starts_with("- ") { 1 } else { body.find(':').map_or(0, |i| i + 1) };
            push(builder, StyleProperty::Brush(SYNTAX), indent..indent + key);
            return decor;
        }
        Block::Heading { level, marker } => style_heading(builder, text, frame.lead, *level, *marker, caret.is_some() || empty, fonts),
        Block::Item(item) => style_item(builder, item, frame.lead, caret, fonts, &mut decor),
        Block::Callout { range, kind } => match (kind, caret.is_some()) {
            (Callout::Typed(index), _) if frame.panels > 0 => style_callout(builder, frame, text, range, KINDS[usize::from(*index)].tone, fonts, &mut decor),
            (Callout::Typed(_), _) => push(builder, StyleProperty::Brush(SYNTAX), range.clone()),
            (_, true) => push(builder, StyleProperty::Brush(SYNTAX), range.clone()),
            (Callout::Quote, false) => hide(builder, range.clone(), 0.0),
            _ => {}
        },
        Block::Code { first, last, fence, lang } => {
            style_code(builder, frame, text, (*first, *last, *fence, *lang), context, &mut decor);
            return decor;
        }
    }
    if decor.inset == 0.0 {
        decor.inset = decor.x + frame.right();
    }
    let room = width.map_or(f64::INFINITY, |w| f64::from(w) - decor.inset);
    style_spans(builder, text, content, caret, context, room, &mut decor);
    decor
}
