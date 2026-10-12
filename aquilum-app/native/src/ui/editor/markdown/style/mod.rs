mod builder;
mod heading;
mod spans;

use std::collections::HashSet;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontStyle, FontWeight, RangedBuilder};

use super::super::highlight::{self, Token};
use super::blocks::{self, Block, Callout};
use super::brushes::{HIDDEN, MARKER, MUTED, SYNTAX, TEXT, TOKENS, token_brush};
use super::metrics::{BOX_CHARS, CHECKBOX, CODE_PADDING, Fonts, LIST_LEVEL, LIST_START, QUOTE_BAR, TAB_SIZE};
use super::Decor;
use builder::{hide, touches};
use heading::style_heading;
use spans::style_spans;

pub use builder::push;

pub fn style(builder: &mut RangedBuilder<'_, BrushIndex>, block: &Block, text: &str, caret: Option<usize>, fonts: &Fonts, unresolved: &HashSet<String>) -> Decor {
    let mut decor = Decor { caret, ..Decor::default() };
    for (i, _) in text.match_indices('\t') {
        hide(builder, i..i + 1, fonts.ch * TAB_SIZE);
    }
    let indent = text.len() - text.trim_start().len();
    let content = block.content().max(indent).min(text.len());
    let empty = text[content..].trim().is_empty();
    match block {
        Block::Text | Block::Table { .. } | Block::Image => {}
        Block::Comment => {
            push(builder, StyleProperty::Brush(MUTED), 0..text.len());
            push(builder, StyleProperty::FontStyle(FontStyle::Italic), 0..text.len());
            return decor;
        }
        Block::Underline => {
            push(builder, StyleProperty::Brush(SYNTAX), 0..text.len());
            return decor;
        }
        Block::Definition { marker } => {
            decor.x = LIST_LEVEL;
            decor.hang = Some(*marker);
            push(builder, StyleProperty::Brush(SYNTAX), 0..*marker);
        }
        Block::Meta => {
            let lead = text.len() - text.trim_start().len();
            let body = &text[lead..];
            let key = if body.starts_with("---") { body.len() } else if body.starts_with("- ") { 1 } else { body.find(':').map_or(0, |i| i + 1) };
            push(builder, StyleProperty::Brush(SYNTAX), lead..lead + key);
            return decor;
        }
        Block::Heading { level, marker } => style_heading(builder, text, 0, *level, *marker, caret.is_some() || empty, fonts),
        Block::Item(item) => {
            let (marker, task, callout) = (item.marker(), item.task(), item.callout());
            decor.x = LIST_START + LIST_LEVEL * item.level() as f64;
            hide(builder, 0..item.indent(), 0.0);
            let symbol = marker.start..marker.end - 1;
            if touches(caret, &symbol) {
                push(builder, StyleProperty::Brush(TEXT), marker.clone());
            } else {
                push(builder, StyleProperty::Brush(if item.ordered { MARKER } else { HIDDEN }), marker.clone());
            }
            if let Some((range, _)) = task
                && !touches(caret, &range)
            {
                hide(builder, range, CHECKBOX / BOX_CHARS as f32);
            }
            if let Some((at, tone)) = callout.filter(|_| fonts.callouts) {
                push(builder, StyleProperty::Brush(BrushIndex(TOKENS + usize::from(tone))), at..at + 1);
                push(builder, StyleProperty::FontWeight(FontWeight::BOLD), at..at + 1);
            }
            decor.hang = Some(marker.end);
        }
        Block::Continuation { level, lead, width } => {
            decor.x = LIST_START + LIST_LEVEL * *level as f64 + f64::from(fonts.ch) * *width as f64;
            hide(builder, 0..*lead, 0.0);
        }
        Block::Quote { marker, callout, .. } => {
            decor.x = QUOTE_BAR;
            let inside = caret.is_some();
            push(builder, StyleProperty::Brush(if inside { SYNTAX } else { HIDDEN }), 0..*marker);
            match callout {
                Some((range, Callout::Quote)) if !inside => hide(builder, range.clone(), 0.0),
                Some((range, _)) if inside => push(builder, StyleProperty::Brush(SYNTAX), range.clone()),
                _ => {}
            }
            if let Some(Block::Heading { level, marker: hashes }) = blocks::heading(&text[*marker..]) {
                style_heading(builder, text, *marker, level, hashes, inside, fonts);
            }
            decor.hang = Some(*marker);
        }
        Block::Rule => {
            push(builder, StyleProperty::Brush(if caret.is_some() { SYNTAX } else { HIDDEN }), 0..text.len());
            return decor;
        }
        Block::Code { first, last, fence, lang } => {
            decor.x = CODE_PADDING;
            decor.inset = CODE_PADDING * 2.0;
            decor.pad = (if *first { CODE_PADDING } else { 0.0 }, if *last { CODE_PADDING } else { 0.0 });
            push(builder, StyleProperty::FontFamily(fonts.mono.clone()), 0..text.len());
            if *fence {
                let ticks = text.len() - text.trim_start_matches([' ', '`', '~']).len();
                push(builder, StyleProperty::Brush(SYNTAX), 0..ticks);
            } else if let Some(lang) = lang {
                for (range, token) in highlight::tokens(text, *lang) {
                    push(builder, StyleProperty::Brush(token_brush(token)), range.clone());
                    if token == Token::Comment {
                        push(builder, StyleProperty::FontStyle(FontStyle::Italic), range);
                    }
                }
            }
            return decor;
        }
    }
    if decor.inset == 0.0 {
        decor.inset = decor.x;
    }
    style_spans(builder, text, content, caret, fonts, unresolved, &mut decor);
    decor
}
