mod color;
mod paint;
mod presets;

use masonry::core::keyboard::KeyState;
use masonry::core::{Layer, TextEvent};

use super::*;
use color::{hsv, rgb};

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
        Size::new(PAD * 2.0 + FIELD.width, PAD * 2.0 + presets::SWATCH + GAP + FIELD.height + GAP + HUE)
    }

    fn field(bounds: Rect) -> Rect {
        Rect::from_origin_size((bounds.x0 + PAD, bounds.y0 + PAD + presets::SWATCH + GAP), FIELD)
    }

    fn choose(&mut self, ctx: &mut EventCtx<'_>, color: masonry::peniko::Color) {
        let rgba = color.to_rgba8();
        (self.hue, self.saturation, self.value) = hsv(rgba.r, rgba.g, rgba.b);
        ctx.submit_action::<Picked>(Picked::Color(self.hex()));
        ctx.request_paint_only();
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

impl Widget for ColorPicker {
    type Action = Picked;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(e) if crate::ui::keymap::pointer::press(e.button) => {
                let at = ctx.local_position(e.state.position);
                let bounds = ctx.content_box();
                if let Some(index) = presets::at(bounds, at) {
                    self.choose(ctx, presets::colors()[index]);
                    ctx.set_handled();
                    return;
                }
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
            && matches!(crate::ui::keymap::command(&[crate::ui::keymap::Scope::Dialog], &k.key, k.modifiers), Some(crate::ui::keymap::Command::Close | crate::ui::keymap::Command::Confirm))
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
        self.draw(ctx, painter);
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
