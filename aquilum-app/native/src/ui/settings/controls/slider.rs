use super::*;

pub struct Slider {
    label: String,
    hint: String,
    value: f64,
    min: f64,
    max: f64,
    step: f64,
    dragging: bool,
    label_line: text::Line,
    hint_line: text::Line,
}

impl Slider {
    pub fn new(label: &str, value: f64, min: f64, max: f64, step: f64, hint: &str) -> NewWidget<Self> {
        NewWidget::new(Slider { label: label.to_owned(), hint: hint.to_owned(), value, min, max, step, dragging: false, label_line: text::Line::default(), hint_line: text::Line::default() })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, value: f64, hint: &str) {
        if this.widget.value != value || this.widget.hint != hint {
            this.widget.value = value;
            hint.clone_into(&mut this.widget.hint);
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    fn line() -> f64 {
        (f64::from(theme::ui_font_settings().size) * crate::ui::tokens::number::FONT_LINE_HEIGHT_NORMAL).round()
    }

    fn track(&self, bounds: Rect) -> (f64, f64, f64) {
        let r = size::SIZE_12 / 2.0;
        let y = bounds.y0 + Self::line() + size::SPACE_4 + size::SIZE_16 / 2.0;
        (bounds.x0 + r, bounds.x1 - r, y)
    }

    fn snap(&self, value: f64) -> f64 {
        let steps = ((value - self.min) / self.step).round();
        (self.min + steps * self.step).clamp(self.min, self.max)
    }

    fn set_from(&mut self, ctx: &mut EventCtx<'_>, x: f64) {
        let (x0, x1, _) = self.track(ctx.content_box());
        let share = ((x - x0) / (x1 - x0).max(1.0)).clamp(0.0, 1.0);
        let value = self.snap(self.min + share * (self.max - self.min));
        self.move_to(ctx, value);
    }

    fn move_to(&mut self, ctx: &mut EventCtx<'_>, value: f64) {
        if (value - self.value).abs() > f64::EPSILON {
            self.value = value;
            ctx.request_paint_only();
            ctx.submit_action::<Changed>(Changed::Value(value));
        }
    }
}

impl Widget for Slider {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Down(down) if crate::ui::keymap::pointer::press(down.button) => {
                self.dragging = true;
                ctx.capture_pointer();
                ctx.request_focus();
                let x = ctx.local_position(down.state.position).x;
                self.set_from(ctx, x);
            }
            PointerEvent::Move(update) if self.dragging => {
                let x = ctx.local_position(update.current.position).x;
                self.set_from(ctx, x);
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.dragging = false,
            _ => {}
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &masonry::core::TextEvent) {
        use crate::ui::keymap::{self, Command, Scope};
        use masonry::core::keyboard::KeyState;
        let masonry::core::TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        let value = match keymap::command(&[Scope::Slider], &k.key, k.modifiers) {
            Some(Command::Increase) => self.value + self.step,
            Some(Command::Decrease) => self.value - self.step,
            Some(Command::First) => self.min,
            Some(Command::Last) => self.max,
            _ => return,
        };
        let value = self.snap(value);
        self.move_to(ctx, value);
        ctx.set_handled();
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        let value = match event.action {
            Action::Increment => self.value + self.step,
            Action::Decrement => self.value - self.step,
            _ => return,
        };
        let value = self.snap(value);
        self.move_to(ctx, value);
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FocusChanged(_) => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.label_line.clear();
                self.hint_line.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::FitContent(space) => space,
                _ => Length::px(size::SIZE_128),
            },
            Axis::Vertical => Length::px(Self::line() + size::SPACE_4 + size::SIZE_16),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let line = Self::line();
        let hint_width = {
            let layout = self.hint_line.layout(ctx, &self.hint, bounds.width(), text::Style::ui(line));
            let width = f64::from(layout.width());
            render_text(painter, Affine::translate((bounds.x1 - width, bounds.y0)), layout, &[t.text_tertiary.into()], true);
            width
        };
        let label = self.label_line.layout(ctx, &self.label, (bounds.width() - hint_width - size::SPACE_8).max(1.0), text::Style::ui(line));
        render_text(painter, Affine::translate((bounds.x0, bounds.y0)), label, &[t.text_secondary.into()], true);
        let (x0, x1, y) = self.track(bounds);
        let half = size::UNIT_3 / 2.0;
        painter.fill(RoundedRect::new(bounds.x0, y - half, bounds.x1, y + half, size::ROUNDED_SM), t.bg_surface_overlay).draw();
        let share = if self.max > self.min { (self.value - self.min) / (self.max - self.min) } else { 0.0 };
        let center = Point::new(x0 + (x1 - x0) * share.clamp(0.0, 1.0), y);
        painter.fill(Circle::new(center, size::SIZE_12 / 2.0), t.text_accent).draw();
        if ctx.is_focus_target() {
            let ring = RoundedRect::from_rect(Rect::new(bounds.x0, y - size::SIZE_16 / 2.0, bounds.x1, y + size::SIZE_16 / 2.0).inflate(size::SPACE_4, size::SPACE_4), size::ROUNDED_SM);
            painter.stroke(&ring, &Stroke::new(size::BORDER_1), t.text_accent).draw();
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        if self.dragging { CursorIcon::Grabbing } else { CursorIcon::Grab }
    }

    fn accessibility_role(&self) -> Role {
        Role::Slider
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
        node.set_numeric_value(self.value);
        node.set_min_numeric_value(self.min);
        node.set_max_numeric_value(self.max);
        node.set_numeric_value_step(self.step);
        node.set_value(self.hint.clone());
        node.add_action(Action::Increment);
        node.add_action(Action::Decrement);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
