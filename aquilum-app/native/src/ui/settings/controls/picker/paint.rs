use masonry::peniko::{Color, ColorStop, Gradient};

use super::color::color;
use super::*;

fn knob(painter: &mut Painter<'_>, at: Point, fill: Color) {
    painter.fill(Circle::new(at, KNOB), fill).draw();
    painter.stroke(&Circle::new(at, KNOB), &Stroke::new(2.0), Color::WHITE).draw();
    painter.stroke(&Circle::new(at, KNOB + 1.0), &Stroke::new(1.0), Color::BLACK.with_alpha(0.25)).draw();
}

impl ColorPicker {
    pub(super) fn draw(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let card = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        box_shadow(painter, card, t.elevation_float);
        painter.fill(card, t.dialog_bg).draw();
        box_shadow(painter, card, t.shadow_ring);

        let current = self.hex();
        for (index, preset) in presets::colors().into_iter().enumerate() {
            let swatch = presets::swatch(bounds, index);
            let circle = Circle::new(swatch.center(), swatch.width() / 2.0);
            painter.fill(circle, preset).draw();
            painter.stroke(&circle, &Stroke::new(1.0), Color::BLACK.with_alpha(0.12)).draw();
            if presets::hex(preset) == current {
                painter.stroke(&Circle::new(swatch.center(), swatch.width() / 2.0 + 3.0), &Stroke::new(2.0), preset).draw();
            }
        }

        let field = Self::field(bounds);
        let shape = RoundedRect::from_rect(field, size::RADIUS_MD);
        painter.fill(shape, color(self.hue, 1.0, 1.0)).draw();
        let across = Gradient::new_linear((field.x0, field.y0), (field.x1, field.y0))
            .with_stops([ColorStop::from((0.0, Color::WHITE)), ColorStop::from((1.0, Color::WHITE.with_alpha(0.0)))]);
        painter.fill(shape, &across).draw();
        let down = Gradient::new_linear((field.x0, field.y0), (field.x0, field.y1))
            .with_stops([ColorStop::from((0.0, Color::BLACK.with_alpha(0.0))), ColorStop::from((1.0, Color::BLACK))]);
        painter.fill(shape, &down).draw();
        ring(painter, shape, t.border_default);
        let at = Point::new(field.x0 + field.width() * self.saturation, field.y0 + field.height() * (1.0 - self.value));
        knob(painter, at, color(self.hue, self.saturation, self.value));

        let bar = Self::hue_bar(bounds);
        let stops: Vec<ColorStop> = (0..=6).map(|i| ColorStop::from((i as f32 / 6.0, color(f64::from(i) * 60.0, 1.0, 1.0)))).collect();
        let rainbow = Gradient::new_linear((bar.x0, bar.y0), (bar.x1, bar.y0)).with_stops(stops.as_slice());
        let bar_shape = RoundedRect::from_rect(bar, HUE / 2.0);
        painter.fill(bar_shape, &rainbow).draw();
        ring(painter, bar_shape, t.border_default);
        let at = Point::new(bar.x0 + bar.width() * self.hue / 360.0, bar.center().y);
        knob(painter, at, color(self.hue, 1.0, 1.0));
    }
}
