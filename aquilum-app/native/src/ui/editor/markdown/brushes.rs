use masonry::core::BrushIndex;
use masonry::peniko::{Brush, Color};

use super::super::highlight::Token;
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

pub fn palette(tm: &Theme, text: Color) -> Vec<Brush> {
    let tones = [
        tm.list_callout_important,
        tm.list_callout_question,
        tm.list_callout_highlight,
        tm.list_callout_idea,
        tm.list_callout_info,
        tm.list_callout_success,
        tm.list_callout_muted,
    ];
    let code = [tm.editor_code_keyword, tm.editor_code_string, tm.editor_code_number, tm.editor_code_comment, tm.editor_code_function, tm.editor_code_type, tm.editor_code_property];
    let extra = [tm.text_secondary, tm.text_danger, tm.text_on_accent, tm.text_muted, tm.text_success];
    [text, Color::TRANSPARENT, tm.text_tertiary, tm.text_link, tm.editor_list_marker_color, tm.text_accent].into_iter().chain(tones).chain(code).chain(extra).map(Brush::from).collect()
}

pub fn tone(theme: &Theme, index: u8) -> Color {
    [
        theme.list_callout_important,
        theme.list_callout_question,
        theme.list_callout_highlight,
        theme.list_callout_idea,
        theme.list_callout_info,
        theme.list_callout_success,
        theme.list_callout_muted,
    ][usize::from(index)]
}

pub(super) fn token_brush(token: Token) -> BrushIndex {
    BrushIndex(TONES + token as usize)
}

