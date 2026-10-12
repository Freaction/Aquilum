use super::*;

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
