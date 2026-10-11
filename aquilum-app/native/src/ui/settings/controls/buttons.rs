use super::*;

pub struct DropdownButton {
    label: String,
    line: text::Line,
    press: Press,
    open: bool,
}

impl DropdownButton {
    pub fn new(label: &str) -> NewWidget<Self> {
        NewWidget::new(DropdownButton { label: label.to_owned(), line: text::Line::default(), press: Press::default(), open: false })
    }

    pub fn set_open(this: &mut WidgetMut<'_, Self>, open: bool) {
        this.widget.open = open;
        this.ctx.request_paint_only();
    }

    pub fn set_label(this: &mut WidgetMut<'_, Self>, label: &str) {
        this.widget.label = label.to_owned();
        this.ctx.request_paint_only();
        this.ctx.request_accessibility_update();
    }
}

fn open_menu(ctx: &mut EventCtx<'_>) {
    let origin = ctx.to_window(Point::ZERO);
    let rect = Rect::from_origin_size(origin, ctx.content_box().size());
    ctx.submit_action::<Changed>(Changed::OpenMenu(rect));
}

impl Widget for DropdownButton {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            open_menu(ctx);
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            open_menu(ctx);
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

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(match axis {
            Axis::Horizontal => size::INPUT_WIDTH,
            Axis::Vertical => size::INPUT_PADDING_Y * 2.0 + size::INPUT_CONTENT_HEIGHT,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let active = ctx.is_hovered() || self.press.down || self.open;
        let bg = if active { t.bg_surface_active } else { t.bg_surface_overlay };
        painter.fill(RoundedRect::from_rect(bounds, size::RADIUS_XL), bg).draw();
        let icon = size::SIZE_20;
        let text_width = bounds.width() - size::INPUT_PADDING_X * 2.0 - icon - size::INPUT_GAP;
        let style = text::Style::ui(size::SIZE_20);
        let layout = self.line.layout(ctx, &self.label, text_width.max(0.0), style);
        let origin = Affine::translate((bounds.x0 + size::INPUT_PADDING_X, bounds.y0 + size::INPUT_PADDING_Y));
        render_text(painter, origin, layout, &[t.text_primary.into()], true);
        let chevron = Point::new(bounds.x1 - size::INPUT_PADDING_X - icon, bounds.center().y - icon / 2.0);
        let glyph = if self.open { icons::CHEVRON_UP } else { icons::CHEVRON_DOWN };
        glyph.draw(painter, chevron, icon, 1.2, t.icon_faint);
    }

    fn accessibility_role(&self) -> Role {
        Role::ComboBox
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.label.clone());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

#[derive(Default)]
pub struct Swatch {
    press: Press,
}

impl Swatch {
    pub fn new() -> NewWidget<Self> {
        NewWidget::new(Swatch::default())
    }
}

impl Widget for Swatch {
    type Action = Changed;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            let rect = ctx.window_transform().transform_rect_bbox(ctx.border_box());
            ctx.submit_action::<Changed>(Changed::OpenColor(rect));
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(size::SIZE_20)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::RADIUS_MD);
        painter.fill(shape, t.bg_accent).draw();
        ring(painter, shape, t.border_default);
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.add_action(Action::Click);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            let rect = ctx.window_transform().transform_rect_bbox(ctx.border_box());
            ctx.submit_action::<Changed>(Changed::OpenColor(rect));
        }
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
