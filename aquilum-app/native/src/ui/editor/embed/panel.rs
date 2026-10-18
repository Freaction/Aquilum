use masonry::core::BrushIndex;
use masonry::imaging::Painter;
use masonry::kurbo::{Point, Rect};
use masonry::peniko::Color;

use super::super::markdown::{ACCENT, DANGER, IDEA, INFO, SECONDARY, SUCCESS, WARNING};
use crate::ui::icons::{self, Icon};
use crate::ui::theme;
use crate::ui::tokens::size;

pub const PAD: f64 = size::GAP_XL;
pub const RADIUS: f64 = size::RADIUS_2XL;
pub const HEADER_PAD: f64 = size::GAP_MD;
pub const ICON: f64 = 20.0;
pub const ICON_GAP: f64 = size::GAP_MD;
pub const BUTTON: f64 = 32.0;
const STROKE: f64 = 1.6;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Surface,
    Accent,
    Info,
    Success,
    Warning,
    Danger,
    Idea,
}

pub struct Kind {
    pub names: &'static [&'static str],
    pub icon: Icon,
    pub tone: Tone,
}

pub const KINDS: &[Kind] = &[
    Kind { names: &["note"], icon: icons::PENCIL, tone: Tone::Accent },
    Kind { names: &["abstract", "summary", "tldr"], icon: icons::CLIPBOARD_LIST, tone: Tone::Info },
    Kind { names: &["info"], icon: icons::INFO, tone: Tone::Accent },
    Kind { names: &["todo"], icon: icons::CIRCLE_CHECK, tone: Tone::Accent },
    Kind { names: &["tip", "hint", "important"], icon: icons::FLAME, tone: Tone::Info },
    Kind { names: &["success", "check", "done"], icon: icons::CHECK, tone: Tone::Success },
    Kind { names: &["question", "help", "faq"], icon: icons::CIRCLE_HELP, tone: Tone::Warning },
    Kind { names: &["warning", "caution", "attention"], icon: icons::TRIANGLE_ALERT, tone: Tone::Warning },
    Kind { names: &["failure", "fail", "missing"], icon: icons::X, tone: Tone::Danger },
    Kind { names: &["danger", "error"], icon: icons::ZAP, tone: Tone::Danger },
    Kind { names: &["bug"], icon: icons::BUG, tone: Tone::Danger },
    Kind { names: &["example"], icon: icons::LIST, tone: Tone::Idea },
    Kind { names: &["cite"], icon: icons::QUOTE, tone: Tone::Surface },
];

pub fn kind(name: &str) -> u8 {
    KINDS.iter().position(|kind| kind.names.contains(&name)).unwrap_or(0) as u8
}

impl Tone {
    fn colors(self) -> (Color, Color) {
        let tm = theme::current();
        match self {
            Tone::Surface => (tm.bg_surface_subtle, tm.icon_muted),
            Tone::Accent => (tm.bg_accent_subtle, tm.icon_accent),
            Tone::Info => (tm.bg_info_subtle, tm.icon_info),
            Tone::Success => (tm.bg_success_subtle, tm.icon_success),
            Tone::Warning => (tm.bg_warning_subtle, tm.icon_warning),
            Tone::Danger => (tm.bg_danger_subtle, tm.icon_danger),
            Tone::Idea => (tm.list_callout_idea.multiply_alpha(0.15), tm.list_callout_idea),
        }
    }

    pub fn fill(self) -> Color {
        self.colors().0
    }

    pub fn brush(self) -> BrushIndex {
        match self {
            Tone::Surface => SECONDARY,
            Tone::Accent => ACCENT,
            Tone::Info => INFO,
            Tone::Success => SUCCESS,
            Tone::Warning => WARNING,
            Tone::Danger => DANGER,
            Tone::Idea => IDEA,
        }
    }
}

pub fn header_pad(line: f32) -> f64 {
    (f64::from(line).max(BUTTON) - f64::from(line)) / 2.0 + HEADER_PAD
}

pub fn header(line: f32) -> f64 {
    f64::from(line) + header_pad(line) * 2.0
}

pub fn digits(lines: usize) -> usize {
    (lines.max(1).ilog10() as usize + 1).max(2)
}

pub fn gutter(digits: usize, ch: f32) -> f64 {
    if digits == 0 { 0.0 } else { digits as f64 * f64::from(ch) + PAD }
}

pub fn button(rect: Rect, header: f64) -> Rect {
    let top = rect.y0 + (header - BUTTON) / 2.0;
    Rect::new(rect.x1 - HEADER_PAD - BUTTON, top, rect.x1 - HEADER_PAD, top + BUTTON)
}

pub struct Header {
    pub height: f64,
    pub icon: Option<Icon>,
    pub button: Option<(Icon, bool, bool)>,
}

pub fn paint(painter: &mut Painter<'_>, rect: Rect, tone: Tone, header: Option<Header>) {
    let (fill, ink) = tone.colors();
    painter.fill(rect.to_rounded_rect(RADIUS), fill).draw();
    let Some(header) = header else { return };
    let tm = theme::current();
    let bottom = rect.y0 + header.height;
    if bottom < rect.y1 {
        painter.fill(Rect::new(rect.x0, bottom - size::BORDER_1, rect.x1, bottom), tm.border_subtle).draw();
    }
    let top = rect.y0 + (header.height - ICON) / 2.0;
    if let Some(icon) = header.icon {
        icon.draw(painter, Point::new(rect.x0 + PAD, top), ICON, STROKE, ink);
    }
    if let Some((icon, hovered, done)) = header.button {
        let (icon, tint) = if done { (icons::CHECK, Some(tm.icon_success)) } else { (icon, None) };
        crate::ui::widgets::paint_icon(painter, button(rect, header.height), icon, hovered, 1.0, tint);
    }
}
