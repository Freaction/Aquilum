use super::*;

impl Widget for ReaderView {
    type Action = ReaderAction;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        self.key_event(ctx, event);
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.pointer_event(ctx, event);
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
        self.arrange(ctx, size);
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
