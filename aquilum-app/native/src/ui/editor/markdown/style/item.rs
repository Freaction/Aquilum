use std::ops::Range;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontWeight, RangedBuilder};

use super::super::Decor;
use super::super::blocks::{Frame, Item};
use super::super::brushes::{HIDDEN, MARKER, TEXT, TOKENS};
use super::super::metrics::{BOX_CHARS, CHECKBOX, Fonts, LIST_LEVEL, LIST_START};
use super::super::reveal::Part;
use super::builder::{hide, push};
use super::indent;

pub(super) fn style_item(builder: &mut RangedBuilder<'_, BrushIndex>, item: &Item, frame: &Frame, selection: Option<&Range<usize>>, fonts: &Fonts, decor: &mut Decor) {
    let (marker, task, callout) = (item.marker(), item.task(), item.callout());
    indent(builder, frame, LIST_START + LIST_LEVEL * item.level() as f64, decor);
    hide(builder, frame.lead..item.indent(), 0.0);
    if Part::Bullet.shown(selection, &item.symbol()) {
        push(builder, StyleProperty::Brush(TEXT), marker.clone());
    } else {
        push(builder, StyleProperty::Brush(if item.ordered { MARKER } else { HIDDEN }), marker.clone());
    }
    if let Some((range, _)) = task
        && !Part::Task.shown(selection, &range)
    {
        hide(builder, range, CHECKBOX / BOX_CHARS as f32);
    }
    if let Some((at, tone)) = callout.filter(|_| fonts.callouts) {
        push(builder, StyleProperty::Brush(BrushIndex(TOKENS + usize::from(tone))), at..at + 1);
        push(builder, StyleProperty::FontWeight(FontWeight::BOLD), at..at + 1);
    }
    decor.hang = Some(marker.end);
}
