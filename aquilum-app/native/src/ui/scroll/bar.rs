//! Геометрия и отрисовка полосы прокрутки по дизайн-токенам.

use masonry::imaging::Painter;
use masonry::kurbo::{Rect, RoundedRect, Size};

use crate::ui::theme;
use crate::ui::tokens::size;

/// Прозрачная рамка вокруг ползунка (`border: var(--q-space-3) solid transparent`).
pub const THUMB_INSET: f64 = 3.0;
/// Минимальная высота ползунка (`min-height: var(--q-size-24)`).
pub const THUMB_MIN: f64 = 24.0;
/// Щелчок по дорожке листает на страницу, как Chromium: 87,5 % видимой высоты.
pub const PAGE: f64 = 0.875;

pub fn track(viewport: Size) -> Rect {
    Rect::new(viewport.width - size::SCROLLBAR_WIDTH, 0.0, viewport.width, viewport.height)
}

pub fn thumb(viewport: Size, content_height: f64, offset: f64, max_offset: f64) -> Rect {
    let t = track(viewport).inflate(-THUMB_INSET, -THUMB_INSET);
    let ratio = (viewport.height / content_height.max(1.0)).min(1.0);
    let height = (t.height() * ratio).max(THUMB_MIN).min(t.height());
    let travel = t.height() - height;
    let top = t.y0 + if max_offset > 0.0 { travel * offset / max_offset } else { 0.0 };
    Rect::new(t.x0, top, t.x1, top + height)
}

pub fn paint_scrollbar(painter: &mut Painter<'_>, track_rect: Rect, thumb_rect: Rect, active: bool) {
    let t = theme::current();
    painter.fill(track_rect, t.scrollbar_track).draw();
    let color = if active { t.scrollbar_thumb_hover } else { t.scrollbar_thumb };
    painter.fill(RoundedRect::from_rect(thumb_rect, thumb_rect.width() / 2.0), color).draw();
}
