use masonry::accesskit::{Action, Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx,
    Widget, WidgetMut, render_text,
};
use masonry::imaging::{Composite, GroupRef, PaintSink, Painter};
use masonry::kurbo::{Affine, Axis, Point, RoundedRect, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::BlendMode;

use super::{ButtonSize, Variant, paint_primary};
use crate::ui::icons::Icon;
use crate::ui::tokens::{number, size};
use crate::ui::widgets::{Press, Pressed};
use crate::ui::{text, theme};

pub struct TextButton {
    label: String,
    variant: Variant,
    pressed: bool,
    icon: Option<Icon>,
    line: text::Line,
    press: Press,
    size: ButtonSize,
}

impl TextButton {
    pub fn new(label: impl Into<String>, variant: Variant) -> Self {
        TextButton { label: label.into(), variant, pressed: false, icon: None, line: text::Line::default(), press: Press::default(), size: ButtonSize::S }
    }

    pub fn size(mut self, size: ButtonSize) -> Self {
        self.size = size;
        self
    }

    pub fn set_disabled(this: &mut WidgetMut<'_, Self>, disabled: bool) {
        this.widget.press.down = false;
        this.ctx.set_disabled(disabled);
        this.ctx.request_paint_only();
    }

    pub fn pressed(mut self, pressed: bool) -> Self {
        self.pressed = pressed;
        self
    }

    pub fn set_pressed(this: &mut WidgetMut<'_, Self>, pressed: bool) {
        this.widget.pressed = pressed;
        this.ctx.request_paint_only();
    }

    pub fn with_icon(mut self, icon: Icon) -> Self {
        self.icon = Some(icon);
        self
    }

    fn icon_width(&self) -> f64 {
        if self.icon.is_some() { size::SIZE_16 + size::SPACE_6 } else { 0.0 }
    }

    fn draw<S: PaintSink + ?Sized>(&mut self, ctx: &mut PaintCtx<'_>, painter: &mut Painter<'_, S>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, self.size.radius());
        let enabled = !ctx.is_disabled();
        let hovered = ctx.is_hovered() && enabled;
        let active = hovered || (self.press.down && enabled) || self.pressed;
        let color = match self.variant {
            Variant::Primary => {
                let shadow = if self.size == ButtonSize::Xs { t.shadow_action_xs } else { t.shadow_action };
                paint_primary(painter, shape, hovered, shadow);
                t.text_on_accent
            }
            Variant::Secondary => {
                painter.fill(shape, if active { t.button_secondary_bg_hover } else { t.button_secondary_bg }).draw();
                if active { t.button_secondary_text_hover } else { t.button_secondary_text }
            }
            Variant::Ghost => {
                if active {
                    painter.fill(shape, t.bg_surface_overlay).draw();
                }
                t.text_tertiary
            }
        };
        let style = self.style();
        let icon_width = self.icon_width();
        let available = (bounds.width() - self.size.padding() * 2.0 - icon_width).max(0.0);
        let layout = self.line.layout(ctx, &self.label, available, style);
        let x = bounds.x0 + (bounds.width() - f64::from(layout.width()) - icon_width) / 2.0;
        let y = bounds.y0 + (bounds.height() - style.line_height) / 2.0;
        if let Some(icon) = &self.icon {
            let glyph = size::SIZE_16;
            icon.draw(painter, Point::new(x, bounds.center().y - glyph / 2.0), glyph, 1.6, color);
        }
        render_text(painter, Affine::translate((x + icon_width, y)), layout, &[color.into()], true);
    }

    fn style(&self) -> text::Style {
        let weight = match self.variant {
            Variant::Primary => number::FONT_WEIGHT_UI_MEDIUM,
            Variant::Secondary | Variant::Ghost => number::FONT_WEIGHT_UI_BASE,
        };
        text::Style { size: size::BUTTON_FONT_SIZE as f32, weight: weight as f32, line_height: self.size.line_height() }
    }
}

impl Widget for TextButton {
    type Action = Pressed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            ctx.submit_action::<Pressed>(Pressed);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<Pressed>(Pressed);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Vertical => Length::px(self.size.height()),
            Axis::Horizontal => {
                let width = text::measure(ctx, &self.label, self.style());
                Length::px(width.ceil() + self.icon_width() + self.size.padding() * 2.0)
            }
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if ctx.is_disabled() {
            let group = GroupRef::new().with_composite(Composite::new(BlendMode::default(), number::OPACITY_50 as f32));
            painter.with_group(group, |painter| self.draw(ctx, painter));
        } else {
            self.draw(ctx, painter);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
