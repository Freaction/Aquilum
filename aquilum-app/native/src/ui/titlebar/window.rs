use super::*;

pub const WINDOW_CHROME: bool = !cfg!(target_os = "android");
pub const OWN_CONTROLS: bool = WINDOW_CHROME && !cfg!(target_os = "macos");

pub struct DragArea;

impl DragArea {
    pub fn new() -> NewWidget<Self> {
        NewWidget::new(DragArea)
    }
}

impl Widget for DragArea {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        drag_or_maximize(ctx, event);
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match len_req {
            LenReq::FitContent(space) => space,
            _ => Length::ZERO,
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

#[derive(Clone, Copy, PartialEq)]
pub enum WindowButtonKind {
    Minimize,
    Maximize,
    Close,
}

pub struct WindowButton {
    kind: WindowButtonKind,
    maximized: bool,
    press: Press,
}

impl WindowButton {
    pub fn new(kind: WindowButtonKind, maximized: bool) -> NewWidget<Self> {
        NewWidget::new(WindowButton { kind, maximized, press: Press::default() })
    }

    pub fn set_maximized(this: &mut WidgetMut<'_, Self>, maximized: bool) {
        if this.widget.maximized != maximized {
            this.widget.maximized = maximized;
            this.ctx.request_paint_only();
            this.ctx.request_accessibility_update();
        }
    }

    fn label(&self) -> String {
        crate::i18n::t(match self.kind {
            WindowButtonKind::Minimize => "window.minimize",
            WindowButtonKind::Maximize => "window.maximize",
            WindowButtonKind::Close => "window.close",
        })
    }

    fn action(&self) -> TitlebarAction {
        match self.kind {
            WindowButtonKind::Minimize => TitlebarAction::Minimize,
            WindowButtonKind::Maximize => TitlebarAction::ToggleMaximize,
            WindowButtonKind::Close => TitlebarAction::CloseWindow,
        }
    }
}

impl Widget for WindowButton {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.press.pointer(ctx, event) {
            ctx.submit_action::<TitlebarAction>(self.action());
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<TitlebarAction>(self.action());
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::HoveredChanged(_)) {
            ctx.request_paint_only();
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(match axis {
            Axis::Horizontal => size::WINDOW_CONTROL_WIDTH,
            Axis::Vertical => size::TABS_HEIGHT,
        })
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let active = ctx.is_hovered() || self.press.down;
        let close = self.kind == WindowButtonKind::Close;
        let (bg, color) = match (active, close) {
            (true, true) => (Some(if self.press.down { t.window_close_bg_active } else { t.window_close_bg_hover }), t.white),
            (true, false) => (Some(t.icon_button_bg_hover), t.icon_button_icon_hover),
            (false, _) => (None, t.icon_button_icon),
        };
        if let Some(bg) = bg {
            painter.fill(bounds, bg).draw();
        }
        let origin = bounds.center() - Vec2::new(6.0, 6.0);
        let at = |x: f64, y: f64| Point::new(origin.x + x, origin.y + y);
        match (self.kind, self.maximized) {
            (WindowButtonKind::Minimize, _) => {
                painter.fill(Rect::from_points(at(1.0, 6.0), at(11.0, 7.0)), color).draw();
            }
            (WindowButtonKind::Maximize, false) => {
                painter.stroke(Rect::from_points(at(1.5, 1.5), at(10.5, 10.5)), &Stroke::new(1.0), color).draw();
            }
            (WindowButtonKind::Maximize, true) => {
                painter.stroke(Rect::from_points(at(1.5, 3.5), at(8.5, 10.5)), &Stroke::new(1.0), color).draw();
                let mut back = BezPath::new();
                back.move_to(at(3.5, 3.0));
                back.line_to(at(3.5, 1.5));
                back.line_to(at(10.5, 1.5));
                back.line_to(at(10.5, 8.5));
                back.line_to(at(9.0, 8.5));
                painter.stroke(&back, &Stroke::new(1.0), color).draw();
            }
            (WindowButtonKind::Close, _) => {
                let mut cross = BezPath::new();
                cross.move_to(at(1.5, 1.5));
                cross.line_to(at(10.5, 10.5));
                cross.move_to(at(10.5, 1.5));
                cross.line_to(at(1.5, 10.5));
                painter.stroke(&cross, &Stroke::new(1.2), color).draw();
            }
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Button
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label());
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub fn window_controls(maximized: bool) -> (NewWidget<impl Widget>, Handle<WindowButton>) {
    let max = WindowButton::new(WindowButtonKind::Maximize, maximized);
    let handle = Handle::of(&max);
    let row = masonry::widgets::Flex::row()
        .with_fixed(WindowButton::new(WindowButtonKind::Minimize, false))
        .with_fixed(max)
        .with_fixed(WindowButton::new(WindowButtonKind::Close, false));
    let underlined = crate::ui::widgets::Underlined::new(NewWidget::new(row), || theme::current().tabs_bg, || theme::current().tabs_border);
    (underlined, handle)
}
