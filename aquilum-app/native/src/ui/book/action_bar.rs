use super::*;

pub struct ActionBar {
    pub(super) row: WidgetPod<Flex>,
}

impl Widget for ActionBar {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.row);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = ctx.context_size();
        let inner = ctx.compute_length(&mut self.row, len_req.into(), context, axis, cross);
        Length::px(inner.get() + size::SPACE_3 * 2.0)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let inner = Size::new(size_.width - size::SPACE_3 * 2.0, size_.height - size::SPACE_3 * 2.0);
        ctx.run_layout(&mut self.row, inner);
        ctx.place_child(&mut self.row, Point::new(size::SPACE_3, size::SPACE_3));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::ROUNDED_XL);
        box_shadow(painter, shape, tm.shadow_action);
        painter.fill(shape, tm.book_actions_bg).draw();
        let inner = RoundedRect::from_rect(ctx.content_box().inset(-0.5), size::ROUNDED_XL - 0.5);
        painter.stroke(&inner, &Stroke::new(size::BORDER_1), tm.book_actions_border).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::Toolbar
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.row.id()])
    }
}
