use super::*;

impl Widget for TextEditor {
    type Action = TextAction;

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, interval: u64) {
        self.do_anim_frame(ctx, props, interval);
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.do_pointer_event(ctx, props, event);
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, props: &mut PropertiesMut<'_>, event: &TextEvent) {
        self.do_text_event(ctx, props, event);
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn accepts_text_input(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, props: &mut PropertiesMut<'_>, event: &Update) {
        self.do_update(ctx, props, event);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        self.do_measure(ctx, props, axis, len_req, cross)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, props: &PropertiesRef<'_>, size: Size) {
        self.do_layout(ctx, props, size);
    }

    fn compose(&mut self, ctx: &mut ComposeCtx<'_>) {
        self.do_compose(ctx);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut masonry::imaging::Painter<'_>) {
        self.do_paint(ctx, props, painter);
    }

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, props: &PropertiesRef<'_>, painter: &mut masonry::imaging::Painter<'_>) {
        self.do_post_paint(ctx, props, painter);
    }

    fn get_cursor(&self, ctx: &QueryCtx<'_>, pos: Point) -> CursorIcon {
        self.do_get_cursor(ctx, pos)
    }

    fn accessibility_role(&self) -> Role {
        if self.single_line { Role::TextInput } else { Role::MultilineTextInput }
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.doc.text());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
