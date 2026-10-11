use super::*;

impl Widget for ReaderView {
    type Action = ReaderAction;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let TextEvent::Keyboard(k) = event else { return };
        if k.state != KeyState::Down {
            return;
        }
        if k.code == Code::KeyC && (k.modifiers.ctrl() || k.modifiers.meta()) && self.popover.is_none() {
            if let Some((from, to)) = self.ordered() {
                let text = self.session.text(from, to);
                if let Err(error) = arboard::Clipboard::new().and_then(|mut c| c.set_text(text)) {
                    report!("не скопировано: {error}");
                }
                ctx.set_handled();
            }
            return;
        }
        match &k.key {
            Key::Named(NamedKey::Escape) => {
                ctx.submit_action::<ReaderAction>(if self.popover.is_some() { ReaderAction::Settings } else { ReaderAction::Close });
            }
            _ if self.popover.is_some() => return,
            Key::Named(NamedKey::ArrowRight | NamedKey::PageDown) => self.turn(ctx, Side::Next),
            Key::Character(c) if c == " " => self.turn(ctx, Side::Next),
            Key::Named(NamedKey::ArrowLeft | NamedKey::PageUp) => self.turn(ctx, Side::Prev),
            _ => return,
        }
        ctx.set_handled();
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        match event {
            PointerEvent::Move(update) => {
                let p = ctx.local_position(update.current.position);
                if self.selecting {
                    if let Some((_, focus)) = self.session.hit(p - self.body.origin().to_vec2(), false)
                        && let Some((anchor, old)) = self.selection
                    {
                        if focus != old {
                            self.selection = Some((anchor, focus));
                            ctx.request_paint_only();
                        }
                    }
                    return;
                }
                let hover = self.body.contains(p).then(|| self.side(p));
                self.set_hover(ctx, hover);
            }
            PointerEvent::Leave(_) => self.set_hover(ctx, None),
            PointerEvent::Down(down) => {
                let p = ctx.local_position(down.state.position);
                if self.popover.is_some() {
                    if self.body.contains(p) && !self.popover_rect.contains(p) {
                        ctx.submit_action::<ReaderAction>(ReaderAction::Settings);
                    }
                    return;
                }
                if self.quote_button.is_some() && self.selection.is_some() && self.toolbar.contains(p) {
                    return;
                }
                if let Some((_, cfi)) = self.labels.iter().find(|(r, _)| r.contains(p)) {
                    ctx.submit_action::<ReaderAction>(ReaderAction::QuoteRef(cfi.clone()));
                    return;
                }
                if let Some(side) = self.side(p) {
                    self.turn(ctx, side);
                    return;
                }
                if !self.body.contains(p) {
                    return;
                }
                let had = self.selection.is_some();
                self.selection = self.session.hit(p - self.body.origin().to_vec2(), down.state.count == 2);
                self.selecting = true;
                ctx.capture_pointer();
                if had != self.selection.is_some() {
                    ctx.request_layout();
                }
                ctx.request_paint_only();
            }
            PointerEvent::Up(_) => {
                if !std::mem::take(&mut self.selecting) {
                    return;
                }
                if self.ordered().is_none() {
                    self.selection = None;
                }
                ctx.request_layout();
            }
            PointerEvent::Scroll(PointerScrollEvent { delta, .. }) => {
                if self.popover.is_some() {
                    return;
                }
                let (dx, dy) = match delta {
                    ScrollDelta::PixelDelta(d) => (d.x / ctx.scale_factor(), d.y / ctx.scale_factor()),
                    ScrollDelta::LineDelta(x, y) => (f64::from(*x) * crate::ui::scroll::LINE, f64::from(*y) * crate::ui::scroll::LINE),
                    _ => (0.0, 0.0),
                };
                if self.settings.scrolled {
                    let changed = self.session.scroll_by(-dy);
                    self.moved(ctx, changed);
                    return;
                }
                self.wheel += if dx.abs() > dy.abs() { -dx } else { -dy };
                if self.wheel.abs() >= WHEEL_STEP {
                    let side = if self.wheel > 0.0 { Side::Next } else { Side::Prev };
                    self.wheel = 0.0;
                    self.turn(ctx, side);
                }
            }
            _ => {}
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let mut running = self.intro.advance(interval);
        for zone in &mut self.zones {
            running |= zone.advance(interval);
        }
        running |= self.bar.advance(interval);
        if running {
            ctx.request_anim_frame();
        }
        ctx.set_opacity(self.intro.value() as f32);
        ctx.request_paint_only();
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            ctx.set_opacity(0.0);
            ctx.request_anim_frame();
        }
        if matches!(event, Update::FontsChanged) {
            self.title_line.clear();
            self.pages_line.clear();
            ctx.request_layout();
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.settings_button);
        ctx.register_child(&mut self.close);
        if let Some(popover) = &mut self.popover {
            ctx.register_child(popover);
        }
        if let Some(button) = &mut self.quote_button {
            ctx.register_child(button);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let line = Self::line_height();
        self.header = (size::SPACE_8 * 2.0 + line * 2.0 + size::SPACE_2).max(size::SPACE_48);
        let button = Size::new(size::SIZE_32, size::SIZE_32);
        let y = ((self.header - size::SIZE_32) / 2.0).round();
        let close_x = size.width - size::SPACE_12 - size::SIZE_32;
        ctx.run_layout(&mut self.close, button);
        ctx.place_child(&mut self.close, Point::new(close_x, y));
        ctx.run_layout(&mut self.settings_button, button);
        ctx.place_child(&mut self.settings_button, Point::new(close_x - size::SPACE_4 - size::SIZE_32, y));
        self.body = Rect::new(0.0, self.header + size::BORDER_1, size.width, size.height - PROGRESS - size::BORDER_1);
        let (fcx, lcx) = ctx.text_contexts();
        self.session.ensure(self.body.size(), &self.settings, fcx, lcx);
        if let Some(popover) = &mut self.popover {
            let width = POPOVER_WIDTH.min(size.width - size::SPACE_24);
            let auto = SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent);
            let measured = ctx.compute_size(popover, auto, LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))));
            ctx.run_layout(popover, Size::new(width, measured.height));
            let origin = Point::new(size.width - size::SPACE_12 - width, self.header + size::GAP_XS);
            ctx.place_child(popover, origin);
            self.popover_rect = Rect::from_origin_size(origin, Size::new(width, measured.height));
        }
        let selected = self.ordered().is_some() && !self.selecting;
        if let Some(button) = &mut self.quote_button {
            ctx.set_stashed(button, !selected);
            if selected {
                let width = TOOLBAR_WIDTH.min(size.width - size::SPACE_24);
                let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
                let measured = ctx.compute_size(button, auto, LayoutSize::maybe(Axis::Horizontal, Some(Length::px(width))));
                ctx.run_layout(button, measured);
                let height = measured.height + size::SPACE_8 * 2.0;
                let left = ((size.width - width) / 2.0).round();
                self.toolbar = Rect::new(left, self.body.y1 - size::SPACE_16 - height, left + width, self.body.y1 - size::SPACE_16);
                let button_x = self.toolbar.x1 - size::SPACE_12 - measured.width;
                ctx.place_child(button, Point::new(button_x, self.toolbar.y0 + size::SPACE_8));
                self.preview = Rect::new(self.toolbar.x0 + size::SPACE_12, self.toolbar.y0, button_x - size::GAP_MD, self.toolbar.y1);
            }
        }
        self.emit(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        self.draw(ctx, painter);
    }

    fn accessibility_role(&self) -> Role {
        Role::Dialog
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(if self.title.is_empty() { t("reader.title") } else { self.title.clone() });
        node.set_modal();
    }

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = vec![self.settings_button.id(), self.close.id()];
        ids.extend(self.popover.as_ref().map(WidgetPod::id));
        ids.extend(self.quote_button.as_ref().map(WidgetPod::id));
        ChildrenIds::from_slice(&ids)
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for ReaderView {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
