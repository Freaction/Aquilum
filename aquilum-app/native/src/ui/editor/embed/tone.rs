use masonry::peniko::{Brush, BrushRef, Color};

use super::super::markdown;
use super::panel;
use crate::ui::theme;

const LIFT: f32 = 0.25;
const GUIDE: f32 = 0.08;
const TINT_DARK: f32 = 0.24;
const TINT_LIGHT: f32 = 0.14;

fn vivid(tone: panel::Tone) -> Color {
    let tm = theme::current();
    let base = match tone {
        panel::Tone::Accent => tm.blue_500,
        panel::Tone::Info => tm.teal_500,
        panel::Tone::Success => tm.green_500,
        panel::Tone::Warning => tm.amber_500,
        panel::Tone::Danger => tm.red_500,
        panel::Tone::Idea => tm.purple_500,
        panel::Tone::Surface => tm.gray_500,
    };
    if !theme::is_dark() {
        return base;
    }
    let [r, g, b, a] = base.components;
    Color::new([r + (1.0 - r) * LIFT, g + (1.0 - g) * LIFT, b + (1.0 - b) * LIFT, a])
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Tone {
    Divider,
    Accent,
    AccentSubtle,
    CoverBg,
    Editor,
    Edge,
    Guide,
    Border,
    Panel(panel::Tone),
    Ink(panel::Tone),
}

impl Tone {
    pub fn color(self) -> Color {
        let tm = theme::current();
        match self {
            Tone::Divider => tm.block_divider,
            Tone::Accent => tm.bg_accent,
            Tone::AccentSubtle => tm.bg_accent_subtle,
            Tone::CoverBg => tm.book_callout_cover_bg,
            Tone::Editor => tm.editor_bg,
            Tone::Edge => tm.icon_secondary,
            Tone::Guide => tm.icon_secondary.multiply_alpha(GUIDE),
            Tone::Border => tm.border_default,
            Tone::Panel(tone) => vivid(tone).multiply_alpha(if theme::is_dark() { TINT_DARK } else { TINT_LIGHT }),
            Tone::Ink(tone) => vivid(tone),
        }
    }
}

#[derive(Clone, Copy, PartialEq)]
pub(super) enum Paint {
    Text,
    Fixed(Color),
    Tone(Tone),
}

impl Paint {
    pub fn brush(self, palette: &[Brush]) -> BrushRef<'_> {
        match self {
            Paint::Text => BrushRef::from(&palette[markdown::TEXT.0]),
            Paint::Fixed(color) => BrushRef::from(color),
            Paint::Tone(tone) => BrushRef::from(tone.color()),
        }
    }
}
