use super::*;

pub struct Switch {
    on: bool,
    enabled: bool,
    label: String,
    press: Press,
    travel: f64,
}

pub(super) const SWITCH_SECONDS: f64 = 0.2;

pub(super) fn ease_standard(t: f64) -> f64 {
    crate::ui::widgets::bezier(crate::ui::widgets::EASE_STANDARD, t)
}

impl Switch {
    pub fn new(on: bool, label: &str) -> NewWidget<Self> {
        NewWidget::new(Switch { on, enabled: true, label: label.to_owned(), press: Press::default(), travel: if on { 1.0 } else { 0.0 } })
    }

    pub fn locked(on: bool, label: &str) -> NewWidget<Self> {
        NewWidget::new(Switch { on, enabled: false, label: label.to_owned(), press: Press::default(), travel: if on { 1.0 } else { 0.0 } })
    }

    pub fn travel(&self) -> f64 {
        self.travel
    }

    pub fn resume(this: &mut WidgetMut<'_, Self>, travel: f64) {
        this.widget.travel = travel;
        if travel != if this.widget.on { 1.0 } else { 0.0 } {
            this.ctx.request_anim_frame();
            this.ctx.request_paint_only();
        }
    }

    fn toggle(&mut self, ctx: &mut EventCtx<'_>) {
        if !self.enabled {
            return;
        }
        self.on = !self.on;
        ctx.request_anim_frame();
        ctx.request_paint_only();
        ctx.request_accessibility_update();
        ctx.submit_action::<Changed>(Changed::Bool(self.on));
    }
}

impl Widget for Switch {
    type Action = Changed;

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let goal = if self.on { 1.0 } else { 0.0 };
        let step = interval as f64 / 1e9 / SWITCH_SECONDS;
        self.travel = if self.travel < goal { (self.travel + step).min(goal) } else { (self.travel - step).max(goal) };
        ctx.request_paint_only();
        if self.travel != goal {
            ctx.request_anim_frame();
        }
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            self.toggle(ctx);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            self.toggle(ctx);
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(match axis {
            Axis::Horizontal => size::SWITCH_WIDTH,
            Axis::Vertical => size::SWITCH_HEIGHT,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let alpha = if self.enabled { 1.0 } else { 0.5 };
        let track = RoundedRect::from_rect(bounds, bounds.height() / 2.0);
        let track_color = if self.on { t.bg_accent } else { t.bg_surface_active };
        painter.fill(track, track_color.multiply_alpha(alpha)).draw();
        let inset = size::SWITCH_HANDLE_INSET;
        let x = bounds.x0 + inset + size::SWITCH_HANDLE_TRAVEL * ease_standard(self.travel);
        let r = size::SWITCH_HANDLE_SIZE / 2.0;
        let handle = Circle::new((x + r, bounds.center().y), r);
        let rect = RoundedRect::from_rect(Rect::from_center_size(handle.center, Size::new(r * 2.0, r * 2.0)), r);
        let glow = |blur, color: masonry::peniko::Color| Shadow { x: 0.0, y: 0.0, blur, spread: 0.0, color: color.multiply_alpha(alpha) };
        box_shadow(painter, rect, &[glow(1.0, t.shadow_sm), glow(4.0, t.shadow_sm), glow(44.0, t.shadow_md)]);
        painter.fill(handle, t.bg_static_white.multiply_alpha(alpha)).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Switch
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.set_toggled(if self.on { Toggled::True } else { Toggled::False });
        if self.enabled {
            node.add_action(Action::Click);
        } else {
            node.set_disabled();
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
