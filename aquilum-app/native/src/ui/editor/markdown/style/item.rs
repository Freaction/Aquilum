use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::{FontWeight, RangedBuilder};

use super::super::Decor;
use super::super::blocks::Item;
use super::super::brushes::{HIDDEN, MARKER, TEXT, TOKENS};
use super::super::metrics::{BOX_CHARS, CHECKBOX, Fonts, LIST_LEVEL, LIST_START};
use super::builder::{hide, push, touches};

pub(super) fn style_item(builder: &mut RangedBuilder<'_, BrushIndex>, item: &Item, from: usize, caret: Option<usize>, fonts: &Fonts, decor: &mut Decor) {
    let (marker, task, callout) = (item.marker(), item.task(), item.callout());
    decor.x += LIST_START + LIST_LEVEL * item.level() as f64;
    hide(builder, from..item.indent(), 0.0);
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
