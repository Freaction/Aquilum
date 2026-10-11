use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{Layer, TextEvent};
use masonry::peniko::{Color, ColorStop, Gradient};

use super::*;

const PAD: f64 = 12.0;
const FIELD: Size = Size::new(208.0, 140.0);
const GAP: f64 = 10.0;
const HUE: f64 = 12.0;
const KNOB: f64 = 7.0;

#[derive(Debug)]
pub enum Picked {
    Color(String),
    Closed(String),
}

#[derive(Clone, Copy, PartialEq)]
enum Part {
    Field,
    Hue,
}

pub struct ColorPicker {
    hue: f64,
    saturation: f64,
    value: f64,
    drag: Option<Part>,
    anchor: Rect,
}

impl ColorPicker {
    pub fn new(hex: &str, anchor: Rect) -> NewWidget<Self> {
        let [r, g, b] = theme::parse_hex(hex).unwrap_or([0x14, 0x71, 0xeb]);
        let (hue, saturation, value) = hsv(r, g, b);
        NewWidget::new(ColorPicker { hue, saturation, value, drag: None, anchor })
    }

    pub fn size() -> Size {
        Size::new(PAD * 2.0 + FIELD.width, PAD * 2.0 + FIELD.height + GAP + HUE)
    }

    fn field(bounds: Rect) -> Rect {
        Rect::from_origin_size((bounds.x0 + PAD, bounds.y0 + PAD), FIELD)
    }

    fn hue_bar(bounds: Rect) -> Rect {
        let field = Self::field(bounds);
        Rect::new(field.x0, field.y1 + GAP, field.x1, field.y1 + GAP + HUE)
    }

    fn hex(&self) -> String {
        let [r, g, b] = rgb(self.hue, self.saturation, self.value);
        format!("#{r:02x}{g:02x}{b:02x}")
    }

    fn pick(&mut self, ctx: &mut EventCtx<'_>, part: Part, at: Point) {
        let bounds = ctx.content_box();
        match part {
            Part::Field => {
                let f = Self::field(bounds);
                self.saturation = ((at.x - f.x0) / f.width()).clamp(0.0, 1.0);
                self.value = 1.0 - ((at.y - f.y0) / f.height()).clamp(0.0, 1.0);
            }
            Part::Hue => {
                let h = Self::hue_bar(bounds);
                self.hue = ((at.x - h.x0) / h.width()).clamp(0.0, 1.0) * 360.0;
            }
        }
        ctx.submit_action::<Picked>(Picked::Color(self.hex()));
        ctx.request_paint_only();
    }

    fn close(&self, ctx: &mut EventCtx<'_>) {
        ctx.submit_action::<Picked>(Picked::Closed(self.hex()));
        ctx.remove_layer(ctx.widget_id());
    }
}

fn hsv(r: u8, g: u8, b: u8) -> (f64, f64, f64) {
    let [r, g, b] = [r, g, b].map(|c| f64::from(c) / 255.0);
    let max = r.max(g).max(b);
    let delta = max - r.min(g).min(b);
    let hue = if delta == 0.0 {
        0.0
    } else if max == r {
        60.0 * ((g - b) / delta).rem_euclid(6.0)
    } else if max == g {
        60.0 * ((b - r) / delta + 2.0)
    } else {
        60.0 * ((r - g) / delta + 4.0)
    };
    (hue, if max == 0.0 { 0.0 } else { delta / max }, max)
}

fn rgb(hue: f64, saturation: f64, value: f64) -> [u8; 3] {
    let c = value * saturation;
    let h = (hue / 60.0).rem_euclid(6.0);
    let x = c * (1.0 - (h.rem_euclid(2.0) - 1.0).abs());
    let (r, g, b) = match h as u32 {
        0 => (c, x, 0.0),
        1 => (x, c, 0.0),
        2 => (0.0, c, x),
        3 => (0.0, x, c),
        4 => (x, 0.0, c),
        _ => (c, 0.0, x),
    };
    let m = value - c;
    [r, g, b].map(|v| ((v + m) * 255.0).round().clamp(0.0, 255.0) as u8)
}

fn color(hue: f64, saturation: f64, value: f64) -> Color {
    let [r, g, b] = rgb(hue, saturation, value);
    Color::from_rgb8(r, g, b)
}

impl Widget for ColorPicker {
    type Action = Picked;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                let at = ctx.local_position(e.state.position);
                let bounds = ctx.content_box();
                let part = if Self::field(bounds).inflate(KNOB, KNOB).contains(at) {
                    Part::Field
                } else if Self::hue_bar(bounds).inflate(KNOB, KNOB).contains(at) {
                    Part::Hue
                } else {
                    return;
                };
                self.drag = Some(part);
                ctx.capture_pointer();
                self.pick(ctx, part, at);
            }
            PointerEvent::Move(update) => {
                if let Some(part) = self.drag {
                    let at = ctx.local_position(update.current.position);
                    self.pick(ctx, part, at);
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.drag = None,
            _ => {}
        }
        ctx.set_handled();
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        if let TextEvent::Keyboard(k) = event
            && k.state == KeyState::Down
            && matches!(k.key, Key::Named(NamedKey::Escape | NamedKey::Enter))
        {
            self.close(ctx);
            ctx.set_handled();
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(Self::size().get_coord(axis))
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let card = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        box_shadow(painter, card, t.elevation_float);
        painter.fill(card, t.dialog_bg).draw();
        box_shadow(painter, card, t.shadow_ring);

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
        let knob = Point::new(field.x0 + field.width() * self.saturation, field.y0 + field.height() * (1.0 - self.value));
        painter.fill(Circle::new(knob, KNOB), color(self.hue, self.saturation, self.value)).draw();
        painter.stroke(&Circle::new(knob, KNOB), &Stroke::new(2.0), Color::WHITE).draw();
        painter.stroke(&Circle::new(knob, KNOB + 1.0), &Stroke::new(1.0), Color::BLACK.with_alpha(0.25)).draw();

        let bar = Self::hue_bar(bounds);
        let stops: Vec<ColorStop> = (0..=6).map(|i| ColorStop::from((i as f32 / 6.0, color(f64::from(i) * 60.0, 1.0, 1.0)))).collect();
        let rainbow = Gradient::new_linear((bar.x0, bar.y0), (bar.x1, bar.y0)).with_stops(stops.as_slice());
        let bar_shape = RoundedRect::from_rect(bar, HUE / 2.0);
        painter.fill(bar_shape, &rainbow).draw();
        ring(painter, bar_shape, t.border_default);
        let knob = Point::new(bar.x0 + bar.width() * self.hue / 360.0, bar.center().y);
        painter.fill(Circle::new(knob, KNOB), color(self.hue, 1.0, 1.0)).draw();
        painter.stroke(&Circle::new(knob, KNOB), &Stroke::new(2.0), Color::WHITE).draw();
        painter.stroke(&Circle::new(knob, KNOB + 1.0), &Stroke::new(1.0), Color::BLACK.with_alpha(0.25)).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::ColorWell
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.hex());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for ColorPicker {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let position = match event {
            PointerEvent::Down(e) => e.state.position,
            _ => return,
        };
        let local = ctx.local_position(position);
        if ctx.border_box().contains(local) || self.anchor.contains(ctx.to_window(local)) {
            return;
        }
        self.close(ctx);
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn hsv_round_trips_hex() {
        for hex in [[0x14, 0x71, 0xeb], [0xe0, 0x45, 0x7b], [0, 0, 0], [255, 255, 255], [0x2a, 0x9d, 0x8f]] {
            let (h, s, v) = super::hsv(hex[0], hex[1], hex[2]);
            assert_eq!(super::rgb(h, s, v), hex);
        }
    }
}
