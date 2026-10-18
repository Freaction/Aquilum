use std::ops::Range;

use masonry::core::{BrushIndex, StyleProperty};
use masonry::parley::RangedBuilder;

use super::super::super::embed::panel::{self, ICON, ICON_GAP, PAD, Tone};
use super::super::Decor;
use super::super::blocks::Frame;
use super::super::metrics::Fonts;
use super::builder::{hide, push};

pub(super) fn style_callout(builder: &mut RangedBuilder<'_, BrushIndex>, frame: &Frame, text: &str, range: &Range<usize>, tone: Tone, fonts: &Fonts, decor: &mut Decor) {
    push(builder, StyleProperty::Brush(tone.brush()), range.start..text.len());
    if text[range.end..].trim().is_empty() {
        let name = range.start + 2..range.start + text[range.clone()].find(']').unwrap_or(2);
        hide(builder, range.start..name.start, 0.0);
        hide(builder, name.end..range.end, 0.0);
    } else {
        hide(builder, range.clone(), 0.0);
    }
    let pad = panel::header_pad(fonts.line);
    decor.x += ICON + ICON_GAP;
    decor.pad = (pad, pad + if frame.closes > 0 { f64::from(frame.closes - 1) * PAD } else { PAD });
}
