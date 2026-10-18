use masonry::core::BrushIndex;
use masonry::peniko::{Brush, Color};

use super::super::highlight::Token;
use crate::ui::palette;
use crate::ui::tokens::Theme;

pub const TEXT: BrushIndex = BrushIndex(0);
pub const HIDDEN: BrushIndex = BrushIndex(1);
pub const SYNTAX: BrushIndex = BrushIndex(2);
pub const LINK: BrushIndex = BrushIndex(3);
pub const MARKER: BrushIndex = BrushIndex(4);
pub const ACCENT: BrushIndex = BrushIndex(5);
pub(super) const TOKENS: usize = 6;
const TONES: usize = TOKENS + 7;
const EXTRA: usize = TONES + 7;
pub const SECONDARY: BrushIndex = BrushIndex(EXTRA);
pub const DANGER: BrushIndex = BrushIndex(EXTRA + 1);
pub const ON_ACCENT: BrushIndex = BrushIndex(EXTRA + 2);
pub const MUTED: BrushIndex = BrushIndex(EXTRA + 3);
pub const SUCCESS: BrushIndex = BrushIndex(EXTRA + 4);
pub const LINK_MISSING: BrushIndex = BrushIndex(EXTRA + 5);
pub const WARNING: BrushIndex = BrushIndex(EXTRA + 6);
pub const INFO: BrushIndex = BrushIndex(EXTRA + 7);
pub const IDEA: BrushIndex = BrushIndex(TOKENS + 3);
const TAGS: usize = EXTRA + 8;

pub(super) const LIST_CALLOUTS: [(char, fn(&Theme) -> Color); 7] = [
    ('!', |t| t.list_callout_important),
    ('?', |t| t.list_callout_question),
    ('&', |t| t.list_callout_highlight),
    ('~', |t| t.list_callout_idea),
    ('@', |t| t.list_callout_info),
    ('$', |t| t.list_callout_success),
    ('%', |t| t.list_callout_muted),
];

pub fn palette(tm: &Theme, text: Color) -> Vec<Brush> {
    let tones = LIST_CALLOUTS.iter().map(|(_, color)| color(tm));
    let code = [tm.editor_code_keyword, tm.editor_code_string, tm.editor_code_number, tm.editor_code_comment, tm.editor_code_function, tm.editor_code_type, tm.editor_code_property];
    let extra = [tm.text_secondary, tm.text_danger, tm.text_on_accent, tm.text_muted, tm.text_success, tm.text_link.multiply_alpha(0.6), tm.text_warning, tm.text_info];
    let tags = palette::NAMES[..palette::TAG_COLORS].iter().filter_map(|name| palette::color(name, tm)).map(|color| blend(color, text, 0.3));
    [text, Color::TRANSPARENT, tm.text_tertiary, tm.text_link, tm.editor_list_marker_color, tm.text_accent].into_iter().chain(tones).chain(code).chain(extra).chain(tags).map(Brush::from).collect()
}

fn blend(color: Color, text: Color, share: f32) -> Color {
    let [r, g, b, _] = color.components;
    let [tr, tg, tb, _] = text.components;
    Color::new([r + (tr - r) * share, g + (tg - g) * share, b + (tb - b) * share, 1.0])
}

pub(super) fn tag_brush(slot: usize) -> BrushIndex {
    BrushIndex(TAGS + slot)
}

pub fn tone(theme: &Theme, index: u8) -> Color {
    (LIST_CALLOUTS[usize::from(index)].1)(theme)
}

pub(super) fn token_brush(token: Token) -> BrushIndex {
    BrushIndex(TONES + token as usize)
}

