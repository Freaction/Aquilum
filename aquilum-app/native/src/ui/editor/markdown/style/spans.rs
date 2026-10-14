use std::collections::HashSet;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontStyle, FontWeight, RangedBuilder};

use super::super::brushes::{ACCENT, LINK, LINK_MISSING, MUTED, SYNTAX, tag_brush};
use super::super::inline::{Kind, hashtags, inline};
use super::super::metrics::{Fonts, INLINE_CODE_PADDING, SCRIPT, TAG_PADDING};
use super::super::{Decor, Link};
use super::builder::{hide, push, touches};
use crate::ui::palette;

pub(super) fn style_spans(builder: &mut RangedBuilder<'_, BrushIndex>, text: &str, content: usize, caret: Option<usize>, fonts: &Fonts, unresolved: &HashSet<String>, decor: &mut Decor) {
    let tags: Vec<_> = hashtags(&text[content..]).into_iter().map(|tag| tag.start + content..tag.end + content).collect();
    for pair in tags.windows(2) {
        let gap = pair[0].end..pair[1].start;
        let between = &text[gap.clone()];
        if between.chars().all(char::is_whitespace) {
            let each = 2.0 * TAG_PADDING / between.chars().count() as f32;
            for (at, c) in between.char_indices() {
                push(builder, StyleProperty::LetterSpacing(each), gap.start + at..gap.start + at + c.len_utf8());
            }
        }
    }
    for tag in tags {
        if !touches(caret, &tag) {
            let slot = fonts.tags.then(|| palette::tag(text[tag.start + 1..tag.end].split('/').next().unwrap_or_default()));
            push(builder, StyleProperty::Brush(slot.map_or(ACCENT, tag_brush)), tag.clone());
            decor.tags.push((tag, slot));
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
