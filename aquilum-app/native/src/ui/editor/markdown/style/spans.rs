use std::collections::HashSet;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontStyle, FontWeight, RangedBuilder};

use super::super::brushes::{ACCENT, LINK, LINK_MISSING, MUTED, SYNTAX};
use super::super::inline::{Kind, hashtags, inline};
use super::super::metrics::{Fonts, INLINE_CODE_PADDING, SCRIPT};
use super::super::{Decor, Link};
use super::builder::{hide, push, touches};

pub(super) fn style_spans(builder: &mut RangedBuilder<'_, BrushIndex>, text: &str, content: usize, caret: Option<usize>, fonts: &Fonts, unresolved: &HashSet<String>, decor: &mut Decor) {
    for tag in hashtags(&text[content..]) {
        let tag = tag.start + content..tag.end + content;
        if !touches(caret, &tag) {
            push(builder, StyleProperty::Brush(ACCENT), tag.clone());
            decor.tags.push(tag);
        }
    }
    for span in inline(&text[content..]) {
        let whole = span.whole.start + content..span.whole.end + content;
        let inner = span.inner.start + content..span.inner.end + content;
        let shown = touches(caret, &whole);
        let reference = inner.start.saturating_sub(1).max(whole.start)..(inner.end + 1).min(whole.end);
        if let Some((target, wiki)) = &span.target {
            let label = if span.kind == Kind::Reader && !shown { reference.clone() } else { inner.clone() };
            decor.links.push(Link { label, target: target.clone(), wiki: *wiki });
        }
        if span.kind == Kind::Reader && !shown {
            push(builder, StyleProperty::Brush(ACCENT), reference.clone());
            hide(builder, whole.start..reference.start, 0.0);
            hide(builder, reference.end..whole.end, 0.0);
            continue;
        }
        let spacing = if matches!(span.kind, Kind::Code | Kind::Math) { INLINE_CODE_PADDING } else { 0.0 };
        for m in [whole.start..inner.start, inner.end..whole.end] {
            if shown {
                push(builder, StyleProperty::Brush(SYNTAX), m);
            } else {
                hide(builder, m, spacing);
            }
        }
        match span.kind {
            Kind::Emphasis => push(builder, StyleProperty::FontStyle(FontStyle::Italic), inner),
            Kind::Strong => push(builder, StyleProperty::FontWeight(FontWeight::BOLD), inner),
            Kind::Strike => push(builder, StyleProperty::Strikethrough(true), inner),
            Kind::Code | Kind::Math => {
                push(builder, StyleProperty::FontFamily(fonts.mono.clone()), inner.clone());
                decor.codes.push(inner);
            }
            Kind::Sub | Kind::Sup => push(builder, StyleProperty::FontSize(fonts.size * SCRIPT), inner),
            Kind::Underline => push(builder, StyleProperty::Underline(true), inner),
            Kind::Escape => {}
            Kind::Highlight => decor.marks.push(inner),
            Kind::Comment => {
                push(builder, StyleProperty::Brush(MUTED), inner.clone());
                push(builder, StyleProperty::FontStyle(FontStyle::Italic), inner);
            }
            Kind::BlockId => push(builder, StyleProperty::Brush(MUTED), inner),
            Kind::Footnote => push(builder, StyleProperty::Brush(ACCENT), inner),
            Kind::Link | Kind::Reader => {
                let missing = span.target.as_ref().is_some_and(|(target, wiki)| *wiki && unresolved.contains(target.trim()));
                let brush = if missing { LINK_MISSING } else { LINK };
                push(builder, StyleProperty::Brush(brush.clone()), inner.clone());
                push(builder, StyleProperty::Underline(true), inner.clone());
                push(builder, StyleProperty::UnderlineBrush(Some(brush)), inner);
            }
        }
    }
}
