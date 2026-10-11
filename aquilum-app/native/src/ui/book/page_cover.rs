use super::*;

pub struct PageCover {
    visual: Visual,
    position: f64,
    repositioning: bool,
    drag: Option<(f64, f64)>,
    actions: WidgetPod<crate::ui::widgets::Appear>,
    book: Option<WidgetPod<TextButton>>,
    show_actions: bool,
    baked: Option<(BakeKey, ImageBrush)>,
    live: Option<ImageBrush>,
}

impl Drop for PageCover {
    fn drop(&mut self) {
        if let Some(marker) = &self.live {
            crate::render::live::forget(marker);
        }
    }
}

pub(super) type BakeKey = (u16, u16, usize, u64, i64);

pub(super) fn theme_id() -> usize {
    theme::generation()
}

pub(super) fn bake(size: Size, scale: f64, draw: impl FnOnce(&mut Painter<'_>)) -> Option<ImageBrush> {
    let mut scene = Scene::new();
    {
        let mut recorder = Painter::new(&mut scene);
        draw(&mut recorder.as_dyn());
    }
    let (dw, dh) = device(Rect::from_origin_size(Point::ORIGIN, size), scale);
    crate::render::offscreen(dw, dh, |painter| replay_transformed(&scene, painter.sink_mut(), Affine::scale(scale)))
}

pub(super) fn cover_body(painter: &mut Painter<'_>, bounds: Rect, visual: &Visual, position: f64, pattern: Option<&ImageBrush>, scale: f64) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, size::BOOK_COVER_RADIUS);
    painter.with_fill_clip(shape, |painter| {
        painter.fill(bounds, tm.bg_sunken).draw();
        match visual {
            Visual::Pattern(id) => {
                painter.fill(bounds, cover::base_color(id)).draw();
                if let Some(image) = pattern {
                    let (w, h) = (f64::from(image.image.width).max(1.0), f64::from(image.image.height).max(1.0));
                    let (kx, ky) = ((bounds.width() * scale / w).max(1.0) / scale, (bounds.height() * scale / h).max(1.0) / scale);
                    painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale_non_uniform(kx, ky));
                }
            }
            Visual::Image(image) => {
                let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
                let k = (bounds.width() / w).max(bounds.height() / h);
                let (dw, dh) = (w * k, h * k);
                let x = bounds.x0 + (bounds.width() - dw) / 2.0;
                let y = bounds.y0 + (bounds.height() - dh) * position / 100.0;
                painter.draw_image(image, Affine::translate((x, y)) * Affine::scale(k));
            }
        }
        if !theme::is_dark() {
            paint_rays(painter, Rect::new(bounds.x1 - RAYS_WIDTH, bounds.y0, bounds.x1, bounds.y1), scale);
        }
    });
}

impl PageCover {
    pub(super) fn new(visual: Visual, position: f64, actions: NewWidget<ActionBar>, book: Option<NewWidget<TextButton>>) -> NewWidget<Self> {
        NewWidget::new(PageCover { visual, position, repositioning: false, drag: None, actions: crate::ui::widgets::Appear::new(actions, crate::ui::widgets::FAST).to_pod(), book: book.map(NewWidget::to_pod), show_actions: false, baked: None, live: None })
    }

    pub fn refresh(this: &mut WidgetMut<'_, Self>) {
        this.ctx.request_paint_only();
    }

    pub fn position(&self) -> f64 {
        self.position
    }

    pub fn repositioning(&self) -> bool {
        self.repositioning
    }

    pub fn set_repositioning(this: &mut WidgetMut<'_, Self>, on: bool, position: Option<f64>) {
        this.widget.repositioning = on;
        if let Some(position) = position {
            this.widget.position = position;
        }
        this.widget.drag = None;
        let show = on || this.ctx.is_hovered() || this.ctx.has_hovered();
        this.widget.show_actions = show;
        this.ctx.set_stashed(&mut this.widget.actions, !show);
        this.ctx.request_paint_only();
        this.ctx.request_layout();
    }

    fn overflow(&self, size_: Size) -> f64 {
        let Visual::Image(image) = &self.visual else { return 0.0 };
        let (w, h) = (f64::from(image.image.width), f64::from(image.image.height));
        if w <= 0.0 || h <= 0.0 {
            return 0.0;
        }
        (h * (size_.width / w).max(size_.height / h) - size_.height).max(0.0)
    }

    fn motion(&self) -> Option<&'static str> {
        match &self.visual {
            Visual::Pattern(id) if cover::animated(id) => Some(id),
            _ => None,
        }
    }

    fn sync_actions(&mut self, ctx: &mut UpdateCtx<'_>) {
        let show = self.repositioning || ctx.is_hovered() || ctx.has_hovered();
        if show != self.show_actions {
            self.show_actions = show;
            ctx.set_stashed(&mut self.actions, !show);
            ctx.request_layout();
        }
    }
}

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

pub(super) fn linear(from: Point, to: Point, stops: &[(f32, Color)]) -> Gradient {
    Gradient::new_linear(from, to).with_stops(stops.iter().map(|(o, c)| ColorStop::from((*o, *c))).collect::<Vec<_>>().as_slice())
}

pub(super) fn cached(kind: u8, width: u32, height: u32, make: impl FnOnce() -> Option<ImageBrush>) -> Option<ImageBrush> {
    static CACHE: std::sync::Mutex<Vec<((u8, u32, u32), ImageBrush)>> = std::sync::Mutex::new(Vec::new());
    let key = (kind, width, height);
    if let Some((_, image)) = CACHE.lock().ok()?.iter().find(|(k, _)| *k == key) {
        return Some(image.clone());
    }
    let image = make()?;
    let mut cache = CACHE.lock().ok()?;
    if cache.len() >= 8 {
        cache.remove(0);
    }
    cache.push((key, image.clone()));
    Some(image)
}

pub(super) fn device(rect: Rect, scale: f64) -> (u16, u16) {
    ((rect.width() * scale).ceil().clamp(1.0, 8192.0) as u16, (rect.height() * scale).ceil().clamp(1.0, 8192.0) as u16)
}

pub(super) fn paint_rays(painter: &mut Painter<'_, impl PaintSink + ?Sized>, rect: Rect, scale: f64) {
    let transform = Affine::translate(rect.origin().to_vec2()) * Affine::scale_non_uniform(rect.width() / 311.0, rect.height() / 186.0);
    let white = Color::WHITE;
    let overlay = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 0.5);
    painter.with_group(GroupRef::new().with_composite(overlay), |painter| {
        let gradient = linear(Point::new(311.0, 93.0), Point::new(0.0, 93.0), &[(0.0, white), (1.0, white.with_alpha(0.0))]);
        painter.fill(Rect::new(0.0, 0.0, 311.0, 186.0), &gradient).transform(transform).draw();
    });
    let (dw, dh) = device(rect, scale);
    let Some(rays) = cached(0, u32::from(dw), u32::from(dh), || {
        crate::render::offscreen(dw, dh, |painter| {
            let local = Affine::scale_non_uniform(f64::from(dw) / 311.0, f64::from(dh) / 186.0);
            let gray = (Color::from_rgb8(0x83, 0x83, 0x83), Color::from_rgb8(0x4d, 0x4d, 0x4d));
            let light = (Color::WHITE, Color::from_rgb8(0x8c, 0x8c, 0x8c));
            let rays: [(&str, f32, f32, (f64, f64, f64, f64), (Color, Color)); 7] = [
                ("M394.487 -2.94443L237.769 -19.0234L189.579 186.356L394.487 -2.94443Z", 30.0, 1.0, (324.26, -47.1334, 360.017, 133.647), gray),
                ("M298.919 -11.7232L266.071 -63.7713L75.1033 167.198L298.919 -11.7232Z", 20.0, 1.0, (316.004, -78.4749, 325.394, 134.579), gray),
                ("M285.505 -41.0939L270.508 -95.0064L6.92272 126.088L285.505 -41.0939Z", 20.0, 1.0, (324.323, -107.045, 322.222, 107.479), gray),
                ("M353.567 -17.1869L300.139 -18.4095L180.048 181.563L353.567 -17.1869Z", 18.7118, 1.0, (314.1, -51.9018, 349.819, 129.038), light),
                ("M392.156 -33.344L369.785 -44.0036L290.1 175.465L392.156 -33.344Z", 9.0, 0.6, (380.534, -81.8194, 411.882, 135.464), light),
                ("M309.19 -44.1356L294.329 -56.8787L86.597 152.525L309.19 -44.1356Z", 10.0, 0.6, (327.289, -93.2525, 336.494, 120.126), light),
                ("M306.296 -73.0126L295.939 -86.5797L23.0478 112.006L306.296 -73.0126Z", 7.5, 0.6, (340.278, -121.243, 337.971, 93.6527), light),
            ];
            for (path, blur, opacity, (x1, y1, x2, y2), (a, b)) in rays {
                let Ok(path) = BezPath::from_svg(path) else { continue };
                let filters = [Filter::blur(blur * (f64::from(dh) / 186.0) as f32)];
                let group = GroupRef::new().with_filters(&filters).with_composite(Composite::new(BlendMode::default(), opacity));
                painter.with_group(group, |painter| {
                    let gradient = linear(Point::new(x1, y1), Point::new(x2, y2), &[(0.0, a), (1.0, b)]);
                    painter.fill(&path, &gradient).transform(local).draw();
                });
            }
        })
    }) else {
        return;
    };
    let dodge = Composite::new(BlendMode::new(Mix::ColorDodge, Compose::SrcOver), 0.7);
    painter.with_group(GroupRef::new().with_composite(dodge).with_clip(ClipRef::fill(rect)), |painter| {
        painter.draw_image(&rays, Affine::translate(rect.origin().to_vec2()) * Affine::scale(1.0 / scale));
    });
}
