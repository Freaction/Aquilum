use masonry::imaging::{PaintSink, Painter};
use masonry::kurbo::{RoundedRect, Stroke};
use masonry::peniko::{Color, Gradient};

use crate::ui::menu::box_shadow;
use crate::ui::theme;
use crate::ui::tokens::{Shadow, number, size};

#[derive(Clone, Copy, PartialEq)]
pub enum Variant {
    Primary,
    Secondary,
    Ghost,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    M,
    S,
    Xs,
}

impl ButtonSize {
    pub(super) fn height(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_HEIGHT,
            ButtonSize::S => size::BUTTON_S_HEIGHT,
            ButtonSize::Xs => size::BUTTON_XS_HEIGHT,
        }
    }

    pub(super) fn padding(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_PADDING_X,
            ButtonSize::S => size::BUTTON_S_PADDING_X,
            ButtonSize::Xs => size::BUTTON_XS_PADDING_X,
        }
    }

    pub(super) fn radius(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_RADIUS,
            ButtonSize::S => size::BUTTON_S_RADIUS,
            ButtonSize::Xs => size::BUTTON_XS_RADIUS,
        }
    }

    pub(super) fn line_height(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_LINE_HEIGHT,
            ButtonSize::S => size::BUTTON_S_LINE_HEIGHT,
            ButtonSize::Xs => size::BUTTON_XS_LINE_HEIGHT,
        }
    }
}

pub fn paint_primary<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, shape: RoundedRect, hovered: bool, shadow: &[Shadow]) {
    let t = theme::current();
    let bounds = shape.rect();
    box_shadow(painter, shape, shadow);
    painter.stroke(&shape, &Stroke::new(1.0), t.bg_accent).draw();
    painter.fill(shape, t.bg_accent).draw();
    let (fill, ring) = if hovered {
        (number::BUTTON_SHINE_FILL_HOVER, number::BUTTON_SHINE_RING_HOVER)
    } else {
        (number::BUTTON_SHINE_FILL_DEFAULT, number::BUTTON_SHINE_RING_DEFAULT)
    };
    let shine = |alpha: f64| Gradient::new_linear((bounds.x0, bounds.y0), (bounds.x0, bounds.y1)).with_stops([Color::WHITE.multiply_alpha(alpha as f32), Color::WHITE.multiply_alpha(0.0)]);
    painter.fill(shape, &shine(fill)).draw();
    let edge = size::BUTTON_HIGHLIGHT_WIDTH;
    let inner = RoundedRect::from_rect(bounds.inset(-edge / 2.0), shape.radii().top_left - edge / 2.0);
    painter.stroke(&inner, &Stroke::new(edge), &shine(ring)).draw();
}
