use super::*;

pub(super) fn tab_style() -> text::Style {
    text::Style { size: size::TABS_FONT_SIZE as f32, weight: 400.0, line_height: size::TAB_CONTENT_HEIGHT }
}

pub(super) fn tab_width(ctx: &mut MeasureCtx<'_>, title: &str, icon: bool) -> f64 {
    let title = text::measure(ctx, title, tab_style()).ceil();
    let icon = if icon { size::SIZE_16 + size::TAB_GAP } else { 0.0 };
    (size::SPACE_12 * 2.0 + icon + title + size::SIZE_32).clamp(size::TAB_MIN_WIDTH, size::TAB_MAX_WIDTH)
}

pub struct TabButton {
    id: Uuid,
    title: String,
    active: bool,
    document: bool,
    icon: Option<icons::Icon>,
    line: text::Line,
    close_hovered: bool,
    down: Option<(PointerButton, f64)>,
}

impl TabButton {
    pub fn new(id: Uuid, title: &str, active: bool, document: bool, icon: Option<icons::Icon>) -> NewWidget<Self> {
        NewWidget::new(TabButton {
            id,
            title: title.to_owned(),
            active,
            document,
            icon,
            line: text::Line::default(),
            close_hovered: false,
            down: None,
        })
    }

    fn close_rect(&self, bounds: Rect) -> Rect {
        let x = bounds.x1 - size::SPACE_12 - CLOSE_SIZE;
        let y = bounds.center().y - CLOSE_SIZE / 2.0;
        Rect::new(x, y, x + CLOSE_SIZE, y + CLOSE_SIZE)
    }

    fn close_visible(&self, hovered: bool) -> bool {
        self.active || hovered
    }
}

impl Widget for TabButton {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let bounds = ctx.content_box();
        match event {
            PointerEvent::Move(update) => {
                let over = self.close_rect(bounds).contains(ctx.local_position(update.current.position));
                if over != self.close_hovered {
                    self.close_hovered = over;
                    ctx.request_paint_only();
                }
            }
            PointerEvent::Leave(_) => {
                self.close_hovered = false;
                ctx.request_paint_only();
            }
            PointerEvent::Down(e) => {
                let at = ctx.local_position(e.state.position);
                match e.button {
                    Some(button @ (PointerButton::Primary | PointerButton::Auxiliary)) => {
                        self.down = Some((button, at.x));
                        ctx.capture_pointer();
                        ctx.request_paint_only();
                    }
                    Some(PointerButton::Secondary) if self.document => {
                        ctx.set_handled();
                        ctx.submit_action::<TitlebarAction>(TitlebarAction::Menu(self.id, ctx.to_window(at)));
                    }
                    _ => {}
                }
            }
            PointerEvent::Up(e) if self.down.is_some_and(|(button, _)| e.button == Some(button)) => {
                let Some((button, start)) = self.down.take() else { return };
                ctx.request_paint_only();
                let at = ctx.local_position(e.state.position);
                if !ctx.is_hovered() || (at.x - start).abs() >= DRAG_THRESHOLD {
                    return;
                }
                let close = button == PointerButton::Auxiliary
                    || (self.close_visible(true) && self.close_rect(bounds).contains(at));
                let action = if close { TitlebarAction::Close(self.id) } else { TitlebarAction::Select(self.id) };
                ctx.submit_action::<TitlebarAction>(action);
            }
            PointerEvent::Cancel(_) => self.down = None,
            _ => {}
        }
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == Action::Click {
            ctx.submit_action::<TitlebarAction>(TitlebarAction::Select(self.id));
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
            Axis::Vertical => Length::px(size::TABS_HEIGHT),
            Axis::Horizontal => Length::px(tab_width(ctx, &self.title, self.icon.is_some())),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let hovered = ctx.is_hovered();
        paint_tab(painter, bounds, self.active, hovered);
        let style = tab_style();
        let mut left = bounds.x0 + size::SPACE_12;
        if let Some(icon) = self.icon {
            let glyph = size::SIZE_16;
            let color = if self.active { t.icon_primary } else { t.icon_secondary };
            icon.draw(painter, Point::new(left, bounds.center().y - glyph / 2.0), glyph, 1.5, color);
            left += glyph + size::TAB_GAP;
        }
        let width = (bounds.x1 - left - size::SPACE_12 - size::SIZE_32).max(0.0);
        let color = if self.active { t.tab_text_active } else { t.tab_text };
        let layout = self.line.layout(ctx, &self.title, width, style);
        let origin = Point::new(left, bounds.center().y - style.line_height / 2.0);
        render_text(painter, Affine::translate(origin.to_vec2()), layout, &[color.into()], true);

        if self.close_visible(hovered) {
            crate::ui::widgets::paint_close(painter, self.close_rect(bounds), self.close_hovered, 1.0);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Tab
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.title.clone());
        node.set_selected(self.active);
        node.add_action(Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub(super) fn paint_tab(painter: &mut Painter<'_>, bounds: Rect, active: bool, hovered: bool) {
    let t = theme::current();
    let body = if active { Rect::new(bounds.x0, bounds.y0, bounds.x1, bounds.y1 + size::BORDER_1) } else { bounds };
    let bg = if active {
        t.tab_bg_active
    } else if hovered {
        t.tab_bg_hover
    } else {
        t.tab_bg
    };
    painter.fill(body, bg).draw();
    painter.fill(Rect::new(bounds.x1 - size::BORDER_1, bounds.y0, bounds.x1, bounds.y1), t.tab_border).draw();
}

pub struct TabRename {
    title: String,
    field: WidgetPod<TextEditor>,
}

impl TabRename {
    pub fn new(title: &str) -> (NewWidget<Self>, Handle<TextEditor>) {
        let field = NewWidget::new(TextEditor::single_line(title).styled(crate::ui::style::tab_text)).with_class(crate::ui::style::TAB_FIELD);
        let handle = Handle::of(&field);
        (NewWidget::new(TabRename { title: title.to_owned(), field: field.to_pod() }), handle)
    }
}

impl Widget for TabRename {
    type Action = masonry::core::NoAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if matches!(event, PointerEvent::Down(_)) {
            ctx.set_handled();
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.field);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Vertical => Length::px(size::TABS_HEIGHT),
            Axis::Horizontal => Length::px(tab_width(ctx, &self.title, false)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let width = (size_.width - size::SPACE_12 * 2.0).max(0.0);
        let auto = SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.field, auto, Size::new(width, size_.height).into());
        let child = Size::new(width, child.height);
        ctx.run_layout(&mut self.field, child);
        ctx.place_child(&mut self.field, Point::new(size::SPACE_12, ((size_.height - child.height) / 2.0).round()));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        paint_tab(painter, ctx.content_box(), true, false);
    }

    fn accessibility_role(&self) -> Role {
        Role::Tab
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.field.id()])
    }
}
