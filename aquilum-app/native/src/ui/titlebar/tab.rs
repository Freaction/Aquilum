use super::*;

pub(super) fn tab_style() -> text::Style {
    text::Style { size: size::TABS_FONT_SIZE as f32, weight: 400.0, line_height: size::TAB_CONTENT_HEIGHT }
}

pub(super) fn tab_width(ctx: &mut MeasureCtx<'_>, line: &mut text::Line, title: &str, icon: bool) -> f64 {
    let (fcx, lcx) = ctx.text_contexts();
    let title = line.width(fcx, lcx, title, tab_style()).ceil();
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
    down: Option<(Option<PointerButton>, f64)>,
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
                if pointer::press(e.button) || pointer::close(e.button) {
                    self.down = Some((e.button, at.x));
                    ctx.capture_pointer();
                    ctx.request_paint_only();
                } else if pointer::context(e.button) && self.document {
                    ctx.set_handled();
                    ctx.submit_action::<TitlebarAction>(TitlebarAction::Menu(self.id, ctx.to_window(at)));
                }
            }
            PointerEvent::Up(e) if self.down.is_some_and(|(button, _)| e.button == button) => {
                let Some((button, start)) = self.down.take() else { return };
                ctx.request_paint_only();
                let at = ctx.local_position(e.state.position);
                if !ctx.is_hovered() || (at.x - start).abs() >= pointer::drag_slop(e.pointer.pointer_type) {
                    return;
                }
                let close = pointer::close(button) || (self.close_visible(true) && self.close_rect(bounds).contains(at));
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
            Axis::Horizontal => Length::px(tab_width(ctx, &mut self.line, &self.title, self.icon.is_some())),
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
