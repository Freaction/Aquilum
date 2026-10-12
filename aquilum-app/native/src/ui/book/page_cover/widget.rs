use super::*;

impl Widget for PageCover {
    type Action = masonry::core::NoAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if !self.repositioning {
            return;
        }
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                self.drag = Some((ctx.local_position(e.state.position).y, self.position));
                ctx.capture_pointer();
            }
            PointerEvent::Move(update) => {
                if let Some((start, origin)) = self.drag {
                    let overflow = self.overflow(ctx.content_box().size());
                    if overflow > 0.0 {
                        let dy = ctx.local_position(update.current.position).y - start;
                        self.position = (origin - dy / overflow * 100.0).clamp(0.0, 100.0);
                        ctx.request_paint_only();
                    }
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => self.drag = None,
            _ => {}
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => {
                ctx.set_stashed(&mut self.actions, true);
            }
            Update::HoveredChanged(_) | Update::ChildHoveredChanged(_) => self.sync_actions(ctx),
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.actions);
        if let Some(book) = &mut self.book {
            ctx.register_child(book);
        }
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        _ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let row = size::SIZE_32 + size::SPACE_3 * 2.0;
        let mut right = size_.width - CONTROLS_RIGHT;
        let auto = SizeDef::new(LenDef::MaxContent, LenDef::MaxContent);
        if let Some(book) = &mut self.book {
            let button = ctx.compute_size(book, auto, size_.into());
            ctx.run_layout(book, button);
            ctx.place_child(book, Point::new(right - button.width, CONTROLS_TOP + (row - button.height) / 2.0));
            right -= button.width + size::SPACE_8;
        }
        if self.show_actions {
            let bar = ctx.compute_size(&mut self.actions, auto, size_.into());
            ctx.run_layout(&mut self.actions, bar);
            ctx.place_child(&mut self.actions, Point::new(right - bar.width, CONTROLS_TOP + (row - bar.height) / 2.0));
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        let pattern = match &self.visual {
            Visual::Pattern(id) => {
                let span = if cover::centered(id) { bounds.width() } else { ctx.window_size().width.max(bounds.width()) };
                let width = ((span * scale / 128.0).ceil() * 128.0) as u32;
                let height = (bounds.height() * scale).ceil() as u32;
                let key = cover::Key { id, width: width.max(1), height: height.max(1), scale: (scale * 1000.0).round() as u32 };
                cover::image(key)
            }
            Visual::Image(_) => None,
        };
        let exact = matches!(self.visual, Visual::Image(_)) || pattern.as_ref().is_some_and(|p| p.exact);
        let ready = matches!(self.visual, Visual::Image(_)) || pattern.is_some();
        let pattern = pattern.map(|p| p.image);
        if self.repositioning || !exact {
            cover_body(painter, bounds, &self.visual, self.position, pattern.as_ref(), scale);
        } else {
            let (dw, dh) = device(bounds, scale);
            let source = match &self.visual {
                Visual::Pattern(id) => id.as_ptr() as u64,
                Visual::Image(image) => image.image.data.id(),
            };
            let key = (dw, dh, theme_id(), source, (self.position * 10.0).round() as i64);
            if self.baked.as_ref().map(|(k, _)| *k) != Some(key) {
                let local = Rect::from_origin_size(Point::ORIGIN, bounds.size());
                let (visual, position) = (&self.visual, self.position);
                self.baked = bake(bounds.size(), scale, |painter| cover_body(painter, local, visual, position, pattern.as_ref(), scale)).map(|image| (key, image.with_quality(ImageQuality::Low)));
            }
            match &self.baked {
                Some((_, image)) => painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale)),
                None => cover_body(painter, bounds, &self.visual, self.position, pattern.as_ref(), scale),
            }
        }
        if let Some(id) = self.motion()
            && ready
        {
            let motion = crate::render::live::Motion { pattern: id, viewport: ctx.window_size().height };
            let marker = self.live.get_or_insert_with(|| crate::render::live::marker(motion));
            crate::render::live::update(marker, motion);
            painter.fill(RoundedRect::from_rect(bounds, size::BOOK_COVER_RADIUS), &*marker).draw();
        }
        if self.repositioning {
            let frame = RoundedRect::from_rect(bounds.inset(-0.5), size::BOOK_COVER_RADIUS);
            painter.stroke(&frame, &Stroke::new(size::BORDER_1), tm.border_accent.multiply_alpha(0.5)).draw();
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        if self.repositioning { CursorIcon::NsResize } else { CursorIcon::Default }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        let mut ids = vec![self.actions.id()];
        ids.extend(self.book.as_ref().map(WidgetPod::id));
        ChildrenIds::from_slice(&ids)
    }
}
