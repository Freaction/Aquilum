mod blocks;
mod brushes;
mod inline;

#[cfg(test)]
mod tests;

use std::ops::Range;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontFamily, FontStyle, FontWeight, RangedBuilder};

pub use blocks::{Block, Callout, classify};
pub use brushes::{ACCENT, DANGER, HIDDEN, LINK, MARKER, MUTED, ON_ACCENT, SECONDARY, SUCCESS, SYNTAX, TEXT, palette, tone};
use brushes::{TOKENS, token_brush};
use inline::{Kind, hashtags, inline};
use super::highlight::{self, Token};

const LIST_START: f64 = 12.0;
const LIST_LEVEL: f64 = 24.0;
const QUOTE_BAR: f64 = 3.0;
const CODE_PADDING: f64 = 12.0;
const INLINE_CODE_PADDING: f32 = 4.0;
const COLLAPSED: f32 = 0.001;
const CHECKBOX: f32 = 20.0;
const TASK_BOX: usize = 3;
const TAB_SIZE: f32 = 4.0;
const H1: f32 = 24.0 / 14.0;
const H2: f32 = 16.0 / 14.0;

#[derive(Clone, Debug, PartialEq)]
pub struct Link {
    pub label: Range<usize>,
    pub whole: Range<usize>,
    pub target: String,
    pub wiki: bool,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct Decor {
    pub x: f64,
    pub inset: f64,
    pub pad: (f64, f64),
    pub hang: Option<usize>,
    pub codes: Vec<Range<usize>>,
    pub tags: Vec<Range<usize>>,
    pub links: Vec<Link>,
    pub caret: Option<usize>,
}


fn hide(builder: &mut RangedBuilder<'_, BrushIndex>, range: Range<usize>, spacing: f32) {
    if range.is_empty() {
        return;
    }
    builder.push(StyleProperty::FontSize(COLLAPSED), range.clone());
    builder.push(StyleProperty::LetterSpacing(spacing), range.clone());
    builder.push(StyleProperty::Brush(HIDDEN), range);
}

fn touches(caret: Option<usize>, range: &Range<usize>) -> bool {
    caret.is_some_and(|c| range.start <= c && c <= range.end)
}

pub struct Fonts {
    pub size: f32,
    pub mono: FontFamily<'static>,
    pub ch: f32,
}

pub fn style(builder: &mut RangedBuilder<'_, BrushIndex>, block: &Block, text: &str, caret: Option<usize>, fonts: &Fonts) -> Decor {
    let mut decor = Decor { caret, ..Decor::default() };
    for (i, _) in text.match_indices('\t') {
        hide(builder, i..i + 1, fonts.ch * TAB_SIZE);
    }
    let content = block.content().min(text.len());
    let empty = text[content..].trim().is_empty();
    match block {
        Block::Text | Block::Table { .. } | Block::Image => {}
        Block::Heading { level, marker } => {
            let scale = match level {
                1 => H1,
                2 => H2,
                _ => 1.0,
            };
            builder.push(StyleProperty::FontSize(fonts.size * scale), ..);
            builder.push(StyleProperty::FontWeight(FontWeight::BOLD), ..);
            if caret.is_some() || empty {
                builder.push(StyleProperty::Brush(SYNTAX), 0..*marker);
            } else {
                hide(builder, 0..*marker, 0.0);
            }
        }
        Block::Item { level, tabs, marker, ordered, task, callout } => {
            decor.x = LIST_START + LIST_LEVEL * *level as f64;
            hide(builder, 0..*tabs, 0.0);
            let symbol = marker.start..marker.end - 1;
            if touches(caret, &symbol) {
                builder.push(StyleProperty::Brush(TEXT), marker.clone());
            } else {
                builder.push(StyleProperty::Brush(if *ordered { MARKER } else { HIDDEN }), marker.clone());
            }
            if let Some((range, _)) = task
                && !touches(caret, &(range.start..range.start + TASK_BOX))
            {
                hide(builder, range.start..range.start + TASK_BOX, CHECKBOX / TASK_BOX as f32);
            }
            if let Some((at, tone)) = callout {
                builder.push(StyleProperty::Brush(BrushIndex(TOKENS + usize::from(*tone))), *at..at + 1);
                builder.push(StyleProperty::FontWeight(FontWeight::BOLD), *at..at + 1);
            }
            decor.hang = Some(marker.end);
        }
        Block::Continuation { level, lead, width } => {
            decor.x = LIST_START + LIST_LEVEL * *level as f64 + f64::from(fonts.ch) * *width as f64;
            hide(builder, 0..*lead, 0.0);
        }
        Block::Quote { marker, callout } => {
            decor.x = QUOTE_BAR;
            let inside = caret.is_some();
            builder.push(StyleProperty::Brush(if inside { SYNTAX } else { HIDDEN }), 0..*marker);
            match callout {
                Some((range, Callout::Quote)) if !inside => hide(builder, range.clone(), 0.0),
                Some((range, _)) if inside => builder.push(StyleProperty::Brush(SYNTAX), range.clone()),
                _ => {}
            }
            decor.hang = Some(*marker);
        }
        Block::Rule => {
            builder.push(StyleProperty::Brush(if caret.is_some() { SYNTAX } else { HIDDEN }), ..);
            return decor;
        }
        Block::Code { first, last, fence, lang } => {
            decor.x = CODE_PADDING;
            decor.inset = CODE_PADDING * 2.0;
            decor.pad = (if *first { CODE_PADDING } else { 0.0 }, if *last { CODE_PADDING } else { 0.0 });
            builder.push(StyleProperty::FontFamily(fonts.mono.clone()), ..);
            if *fence {
                let ticks = text.len() - text.trim_start_matches([' ', '`', '~']).len();
                builder.push(StyleProperty::Brush(SYNTAX), 0..ticks);
            } else if let Some(lang) = lang {
                for (range, token) in highlight::tokens(text, *lang) {
                    builder.push(StyleProperty::Brush(token_brush(token)), range.clone());
                    if token == Token::Comment {
                        builder.push(StyleProperty::FontStyle(FontStyle::Italic), range);
                    }
                }
            }
            return decor;
        }
    }
    if decor.inset == 0.0 {
        decor.inset = decor.x;
    }
    for tag in hashtags(&text[content..]) {
        let tag = tag.start + content..tag.end + content;
        if !touches(caret, &tag) {
            builder.push(StyleProperty::Brush(ACCENT), tag.clone());
            decor.tags.push(tag);
        }
    }
    for span in inline(&text[content..]) {
        let whole = span.whole.start + content..span.whole.end + content;
        let inner = span.inner.start + content..span.inner.end + content;
        let shown = touches(caret, &whole);
        if let Some((target, wiki)) = &span.target
            && !shown
        {
            decor.links.push(Link { label: inner.clone(), whole: whole.clone(), target: target.clone(), wiki: *wiki });
        }
        if span.kind == Kind::Reader && !shown {
            builder.push(StyleProperty::Brush(ACCENT), whole.start..inner.end + 1);
            hide(builder, inner.end + 1..whole.end, 0.0);
            continue;
        }
        let spacing = if span.kind == Kind::Code { INLINE_CODE_PADDING } else { 0.0 };
        for m in [whole.start..inner.start, inner.end..whole.end] {
            if shown {
                builder.push(StyleProperty::Brush(SYNTAX), m);
            } else {
                hide(builder, m, spacing);
            }
        }
        match span.kind {
            Kind::Emphasis => builder.push(StyleProperty::FontStyle(FontStyle::Italic), inner),
            Kind::Strong => builder.push(StyleProperty::FontWeight(FontWeight::BOLD), inner),
            Kind::Strike => builder.push(StyleProperty::Strikethrough(true), inner),
            Kind::Code => {
                builder.push(StyleProperty::FontFamily(fonts.mono.clone()), inner.clone());
                decor.codes.push(inner);
            }
            Kind::Link | Kind::Reader => {
                builder.push(StyleProperty::Brush(LINK), inner.clone());
                builder.push(StyleProperty::Underline(true), inner.clone());
                builder.push(StyleProperty::UnderlineBrush(Some(LINK)), inner);
            }
        }
    }
    decor
}
