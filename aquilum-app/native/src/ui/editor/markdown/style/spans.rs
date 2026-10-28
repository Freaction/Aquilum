use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontStyle, FontWeight, InlineBox, InlineBoxKind, RangedBuilder, VerticalAlign};

use super::super::super::embed::{MediaView, formula, media};
use super::super::super::resolve::{Fetch, Media, Resolver};
use super::super::brushes::{ACCENT, LINK, LINK_MISSING, MUTED, SYNTAX, tag_brush};
use super::super::inline::{Kind, hashtags, inline};
use super::super::metrics::{INLINE_CODE_PADDING, SCRIPT, TAG_PADDING};
use super::super::{Boxed, Decor, Link};
use super::Context;
use super::super::reveal::Part;
use super::builder::{hide, push};
use crate::ui::palette;

fn picture(builder: &mut RangedBuilder<'_, BrushIndex>, source: &str, at: usize, shown: bool, resolver: &dyn Resolver, room: f64, decor: &mut Decor) {
    let line = media::parse(source);
    let (fetched, stamp) = resolver.media(&line.src, line.wiki);
    decor.pictures.push((line.src.clone(), line.wiki, stamp));
    let Fetch::Ready(Media { image: Some(image), .. }) = &fetched else { return };
    if shown {
        return;
    }
    let image = image.clone();
    let view = MediaView::new(&fetched, line, room, false);
    let (width, height) = (view.frame.width() as f32, view.frame.height() as f32);
    hide(builder, at..at + source.len(), 0.0);
    builder.push_inline_box(InlineBox { id: decor.boxes.len() as u64, kind: InlineBoxKind::InFlow, index: at, width, height, baseline: Some(height), vertical_align: VerticalAlign::BASELINE });
    decor.boxes.push(Boxed::Image { image, crop: view.crop() });
}

pub(super) fn style_spans(builder: &mut RangedBuilder<'_, BrushIndex>, text: &str, content: usize, context: &Context, room: f64, decor: &mut Decor) {
    let (fonts, unresolved) = (context.fonts, context.unresolved);
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
        if !decor.shown(Part::Tag, tag.clone()) {
            let slot = fonts.tags.then(|| palette::tag(text[tag.start + 1..tag.end].split('/').next().unwrap_or_default()));
            push(builder, StyleProperty::Brush(slot.map_or(ACCENT, tag_brush)), tag.clone());
            decor.tags.push((tag, slot));
        }
    }
    let spans = inline(&text[content..]);
    let links: Vec<_> = spans.iter().filter(|s| matches!(s.kind, Kind::Link | Kind::Reader)).map(|s| s.whole.start + content..s.whole.end + content).collect();
    for span in spans {
        let whole = span.whole.start + content..span.whole.end + content;
        let inner = span.inner.start + content..span.inner.end + content;
        let shown = decor.shown(Part::Inline, whole.clone());
        if span.kind == Kind::Image {
            let shown = shown || links.iter().any(|link| link.start <= whole.start && whole.end <= link.end && decor.shown(Part::Inline, link.clone()));
            if let Some(resolver) = context.resolver {
                picture(builder, &text[whole.clone()], whole.start, shown, resolver, room, decor);
            }
            continue;
        }
        let reference = inner.start.saturating_sub(1).max(whole.start)..(inner.end + 1).min(whole.end);
        if let Some((target, wiki)) = &span.target {
            let label = if span.kind == Kind::Reader && !shown { reference.clone() } else { inner.clone() };
            decor.links.push(Link { label, target: target.clone(), wiki: *wiki });
        }
        if span.kind == Kind::Math {
            match (!shown).then(|| formula(&text[inner.clone()], f64::from(fonts.size), inner.start - whole.start > 1)).flatten() {
                Some(formula) => {
                    hide(builder, whole.clone(), 0.0);
                    builder.push_inline_box(InlineBox { id: decor.boxes.len() as u64, kind: InlineBoxKind::InFlow, index: whole.start, width: formula.width, height: formula.ascent + formula.depth, baseline: Some(formula.ascent), vertical_align: VerticalAlign::BASELINE });
                    decor.boxes.push(Boxed::Formula(formula));
                }
                None => {
                    push(builder, StyleProperty::Brush(SYNTAX), whole.start..inner.start);
                    push(builder, StyleProperty::Brush(SYNTAX), inner.end..whole.end);
                }
            }
            continue;
        }
        if span.kind == Kind::Reader && !shown {
            push(builder, StyleProperty::Brush(ACCENT), reference.clone());
            hide(builder, whole.start..reference.start, 0.0);
            hide(builder, reference.end..whole.end, 0.0);
            continue;
        }
        let spacing = if span.kind == Kind::Code { INLINE_CODE_PADDING } else { 0.0 };
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
            Kind::Code => {
                push(builder, StyleProperty::FontFamily(fonts.mono.clone()), inner.clone());
                decor.codes.push(inner);
            }
            Kind::Sub | Kind::Sup => push(builder, StyleProperty::FontSize(fonts.size * SCRIPT), inner),
            Kind::Underline => push(builder, StyleProperty::Underline(true), inner),
            Kind::Escape | Kind::Math | Kind::Image => {}
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
