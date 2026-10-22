use masonry::core::{BrushIndex, render_text};
use masonry::imaging::{Composite, GroupRef, PaintSink, Painter};
use masonry::kurbo::{Affine, Line, Point, Rect, RoundedRect, Stroke};
use masonry::parley::Layout;
use masonry::peniko::{BlendMode, Color, Compose, Gradient, Mix};

use crate::ui::icons::Icon;
use crate::ui::menu::box_shadow;
use crate::ui::tokens::{Shadow, number, size};
use crate::ui::{text, theme};

#[derive(Clone, Copy, PartialEq)]
pub enum Variant {
    Primary,
    Secondary,
    Ghost,
    Link,
}

#[derive(Clone, Copy, PartialEq)]
pub enum ButtonSize {
    M,
    S,
    Xs,
}

#[derive(Clone, Copy)]
pub struct Look {
    pub variant: Variant,
    pub size: ButtonSize,
    pub hovered: bool,
    pub active: bool,
}

impl ButtonSize {
    pub fn height(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_HEIGHT,
            ButtonSize::S => size::BUTTON_S_HEIGHT,
            ButtonSize::Xs => size::BUTTON_XS_HEIGHT,
        }
    }

    fn padding(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_PADDING_X,
            ButtonSize::S => size::BUTTON_S_PADDING_X,
            ButtonSize::Xs => size::BUTTON_XS_PADDING_X,
        }
    }

    fn radius(self) -> f64 {
        match self {
            ButtonSize::M => size::BUTTON_RADIUS,
            ButtonSize::S => size::BUTTON_S_RADIUS,
            ButtonSize::Xs => size::BUTTON_XS_RADIUS,
        }
    }

    pub fn text(self) -> text::Style {
        let line_height = match self {
            ButtonSize::M => size::BUTTON_LINE_HEIGHT,
            ButtonSize::S => size::BUTTON_S_LINE_HEIGHT,
            ButtonSize::Xs => size::BUTTON_XS_LINE_HEIGHT,
        };
        text::Style { size: size::BUTTON_FONT_SIZE as f32, weight: number::FONT_WEIGHT_UI_BASE as f32, line_height }
    }

    pub fn chrome(self, icons: usize) -> f64 {
        (self.padding() + size::BUTTON_LABEL_PADDING_X) * 2.0 + icons as f64 * size::BUTTON_ICON_SIZE
    }
}

pub fn paint_button<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, bounds: Rect, look: Look, layout: &Layout<BrushIndex>, icons: &[Option<Icon>; 2]) {
    let t = theme::current();
    let shape = RoundedRect::from_rect(bounds, look.size.radius());
    let (text_color, icon_color) = match look.variant {
        Variant::Primary => {
            let shadow = if look.size == ButtonSize::Xs { t.shadow_action_xs } else { t.shadow_action };
            paint_primary(painter, shape, look.hovered, shadow);
            (t.text_on_accent, t.icon_on_accent)
        }
        Variant::Secondary => {
            painter.fill(shape, if look.active { t.button_secondary_bg_hover } else { t.button_secondary_bg }).draw();
            (t.button_secondary_text, t.icon_muted)
        }
        Variant::Ghost => {
            if look.active {
                painter.fill(shape, t.bg_surface_overlay).draw();
            }
            (t.text_tertiary, t.icon_muted)
        }
        Variant::Link => (t.text_link, t.icon_accent),
    };
    let width = f64::from(layout.width());
    let (icon, label, line_height) = (size::BUTTON_ICON_SIZE, size::BUTTON_LABEL_PADDING_X, look.size.text().line_height);
    let chrome = look.size.chrome(icons.iter().flatten().count());
    let x = bounds.x0 + (bounds.width() - width - chrome) / 2.0 + look.size.padding();
    let [before, after] = icons;
    let text = Point::new(x + if before.is_some() { icon } else { 0.0 } + label, bounds.y0 + (bounds.height() - line_height) / 2.0);
    for (glyph, at) in [(before, x), (after, text.x + width + label)] {
        if let Some(glyph) = glyph {
            glyph.draw(painter, Point::new(at, bounds.center().y - icon / 2.0), icon, number::BUTTON_ICON_STROKE, icon_color);
        }
    }
    render_text(painter, Affine::translate(text.to_vec2()), layout, &[text_color.into()], true);
    if look.variant == Variant::Link && look.active {
        let y = text.y + line_height - 2.0;
        painter.stroke(Line::new((text.x, y), (text.x + width, y)), &Stroke::new(1.0), text_color).draw();
    }
}

pub fn paint_primary<S: PaintSink + ?Sized>(painter: &mut Painter<'_, S>, shape: RoundedRect, hovered: bool, shadow: &[Shadow]) {
    let t = theme::current();
    let (bounds, radius, edge) = (shape.rect(), shape.radii().top_left, size::BUTTON_HIGHLIGHT_WIDTH);
    let inset = |by: f64| RoundedRect::from_rect(bounds.inset(-by), radius - by);
    box_shadow(painter, shape, shadow);
    painter.fill(shape, t.bg_accent).draw();
    painter.stroke(&inset(edge / 2.0), &Stroke::new(edge), t.icon_accent).draw();
    let (fill, ring, mix) = if hovered {
        (number::BUTTON_SHINE_FILL_HOVER, number::BUTTON_SHINE_RING_HOVER, Mix::Overlay)
    } else {
        (number::BUTTON_SHINE_FILL_DEFAULT, number::BUTTON_SHINE_RING_DEFAULT, Mix::Normal)
    };
    let shine = |alpha: f64| Gradient::new_linear((bounds.x0, bounds.y0), (bounds.x0, bounds.y1)).with_stops([Color::WHITE.multiply_alpha(alpha as f32), Color::WHITE.multiply_alpha(0.0)]);
    let group = GroupRef::new().with_composite(Composite::new(BlendMode::new(mix, Compose::SrcOver), 1.0));
    painter.with_group(group, |painter| painter.fill(inset(edge * 2.0), &shine(fill)).draw());
    painter.stroke(&inset(edge * 1.5), &Stroke::new(edge), &shine(ring)).draw();
}
