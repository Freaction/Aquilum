use std::ops::Range;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::RangedBuilder;

use super::super::brushes::HIDDEN;
use super::super::metrics::COLLAPSED;

pub fn push(builder: &mut RangedBuilder<'_, BrushIndex>, property: StyleProperty, range: Range<usize>) {
    if !range.is_empty() {
        builder.push(property, range);
    }
}

pub(super) fn hide(builder: &mut RangedBuilder<'_, BrushIndex>, range: Range<usize>, spacing: f32) {
    push(builder, StyleProperty::FontSize(COLLAPSED), range.clone());
    push(builder, StyleProperty::LetterSpacing(spacing), range.clone());
    push(builder, StyleProperty::Brush(HIDDEN), range);
}
