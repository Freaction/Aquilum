use super::*;

pub(super) const STAR_PATH: &str = "M26.7326 32.142C34.8108 26.881 36.851 0 36.851 0C36.851 0 38.6803 24.7488 46 30C54.5 36.0979 96 38.5621 96 38.5621C96 38.5621 53.7749 39.0195 45.6578 44.4024C37.5407 49.7852 36.851 63 36.851 63C36.851 63 35.1929 49.7069 26.7326 44.4024C18.7456 39.3946 0 38.5621 0 38.5621C0 38.5621 18.8107 37.3012 26.7326 32.142Z";

pub struct Star;

impl Widget for Star {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        let (dw, dh) = device(bounds, scale);
        let Some(image) = cached(1, u32::from(dw), u32::from(dh), || {
            crate::render::offscreen(dw, dh, |painter| {
                let Ok(star) = BezPath::from_svg(STAR_PATH) else { return };
                let k = f64::from(dw) / 96.0;
                let star = Affine::scale(k) * star;
                painter.fill(&star, Color::WHITE.with_alpha(0.4)).draw();
                let shadows = [(Vec2::new(-1.93416, -1.93416), 1.93416, 0.5), (Vec2::new(1.93416, 11.0), 3.86832, 0.3)];
                let frame = Rect::new(0.0, 0.0, f64::from(dw), f64::from(dh));
                painter.with_fill_clip(&star, |painter| {
                    for (offset, blur, alpha) in shadows {
                        let mut outside = frame.inflate(frame.width(), frame.height()).to_path(0.1);
                        outside.extend(Affine::translate(offset * k) * star.clone());
                        let filters = [Filter::blur((blur * k) as f32)];
                        painter.with_group(GroupRef::new().with_filters(&filters), |painter| {
                            painter.fill(&outside, Color::WHITE.with_alpha(alpha)).fill_rule(Fill::EvenOdd).draw();
                        });
                    }
                });
            })
        }) else {
            return;
        };
        painter.draw_image(&image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(1.0 / scale));
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct Thumbnail {
    pub(super) image: ImageBrush,
    pub(super) height: f64,
    pub(super) replace: WidgetPod<crate::ui::widgets::Appear>,
    pub(super) hovered: bool,
    pub(super) baked: Option<((u16, u16, usize), ImageBrush)>,
}

pub(super) const THUMB_MARGIN: f64 = 48.0;

pub(super) fn thumbnail_body(painter: &mut Painter<'_>, bounds: Rect, image: &ImageBrush) {
    let tm = theme::current();
    let shape = RoundedRect::from_rect(bounds, size::BOOK_THUMB_RADIUS);
    box_shadow(painter, shape, tm.book_thumb_shadow);
    painter.with_fill_clip(shape, |painter| {
        painter.fill(bounds, tm.bg_surface_subtle).draw();
        let h = f64::from(image.image.height);
        if h > 0.0 {
            let k = bounds.height() / h;
            painter.draw_image(image, Affine::translate(bounds.origin().to_vec2()) * Affine::scale(k));
        }
        let overlay = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 1.0);
        if !theme::is_dark() {
            painter.with_group(GroupRef::new().with_composite(overlay), |painter| {
                let diagonal = bounds.width().max(bounds.height());
                let gradient = linear(
                    Point::new(bounds.x1, bounds.y0),
                    Point::new(bounds.x1 - diagonal * 0.7071, bounds.y0 + diagonal * 0.7071),
                    &[(0.0, Color::WHITE), (1.0, Color::WHITE.with_alpha(0.0))],
                );
                painter.fill(bounds, &gradient).draw();
            });
        }
        let wash = Composite::new(BlendMode::new(Mix::Overlay, Compose::SrcOver), 0.5);
        painter.with_group(GroupRef::new().with_composite(wash), |painter| {
            let view = Affine::translate(bounds.origin().to_vec2()) * Affine::scale_non_uniform(bounds.width() / 130.0, bounds.height() / 166.0);
            painter.fill(Rect::new(0.0, 0.0, 130.0, 166.0), Color::WHITE).transform(view).draw();
            let radial = Gradient::new_radial(Point::ORIGIN, 1.0).with_stops(
                [
                    ColorStop::from((0.052_083_3, Color::from_rgba8(0x00, 0xff, 0x84, 84))),
                    ColorStop::from((0.447_931, Color::from_rgba8(0x00, 0x75, 0xff, 97))),
                    ColorStop::from((1.0, Color::WHITE.with_alpha(0.0))),
                ]
                .as_slice(),
            );
            let brush = Affine::translate((65.0, 0.0)) * Affine::rotate(std::f64::consts::FRAC_PI_2) * Affine::scale_non_uniform(166.0, 178.164);
            painter.fill(Rect::new(0.0, 0.0, 130.0, 166.0), &radial).transform(view).brush_transform(Some(brush)).draw();
        });
    });
    let border = RoundedRect::from_rect(bounds.inset(-0.5), size::BOOK_THUMB_RADIUS - 0.5);
    painter.stroke(&border, &Stroke::new(size::BORDER_1), tm.book_thumb_border).draw();
}

impl Widget for Thumbnail {
    type Action = ThumbAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(_) = event {
            ctx.request_focus();
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let TextEvent::Keyboard(key) = event else { return };
        if key.state != KeyState::Down {
            return;
        }
        let action = key.modifiers.ctrl() || key.modifiers.meta();
        match &key.key {
            Key::Named(NamedKey::Backspace | NamedKey::Delete) => {
                ctx.submit_action::<ThumbAction>(ThumbAction::Remove);
                ctx.set_handled();
            }
            Key::Named(NamedKey::Escape) => {
                ctx.resign_focus();
                ctx.set_handled();
            }
            Key::Character(c) if action && c.eq_ignore_ascii_case("v") => {
                ctx.submit_action::<ThumbAction>(ThumbAction::Paste);
                ctx.set_handled();
            }
            _ => {}
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded => ctx.set_stashed(&mut self.replace, true),
            Update::HoveredChanged(_) | Update::ChildHoveredChanged(_) => {
                let hovered = ctx.is_hovered() || ctx.has_hovered();
                if hovered != self.hovered {
                    self.hovered = hovered;
                    ctx.set_stashed(&mut self.replace, !hovered);
                    ctx.request_layout();
                }
            }
            Update::FocusChanged(_) => ctx.request_paint_only(),
            _ => {}
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.replace);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        self.height = size_.height;
        if self.hovered {
            let button = Size::new(size::SIZE_32, size::SIZE_32);
            ctx.run_layout(&mut self.replace, button);
            let inset = size::BOOK_THUMB_ACTION_INSET;
            ctx.place_child(&mut self.replace, Point::new(size_.width - inset - button.width, inset));
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let bounds = ctx.content_box();
        let scale = ctx.scale_factor();
        if ctx.is_focus_target() {
            box_shadow(painter, RoundedRect::from_rect(bounds, size::BOOK_THUMB_RADIUS), tm.shadow_focus_ring);
        }
        let canvas = bounds.size() + Size::new(THUMB_MARGIN * 2.0, THUMB_MARGIN * 2.0);
        let (dw, dh) = device(Rect::from_origin_size(Point::ORIGIN, canvas), scale);
        let key = (dw, dh, theme_id());
        if self.baked.as_ref().map(|(k, _)| *k) != Some(key) {
            let local = Rect::from_origin_size(Point::new(THUMB_MARGIN, THUMB_MARGIN), bounds.size());
            let image = &self.image;
            self.baked = bake(canvas, scale, |painter| thumbnail_body(painter, local, image)).map(|image| (key, image));
        }
        match &self.baked {
            Some((_, image)) => {
                let origin = bounds.origin() - Vec2::new(THUMB_MARGIN, THUMB_MARGIN);
                painter.draw_image(image, Affine::translate(origin.to_vec2()) * Affine::scale(1.0 / scale));
            }
            None => thumbnail_body(painter, bounds, &self.image),
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Image
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(t("book.coverAria"));
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.replace.id()])
    }
}
