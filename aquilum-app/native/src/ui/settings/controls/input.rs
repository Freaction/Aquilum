use super::*;

#[derive(Debug)]
pub struct Committed;

pub struct InputBox<W: Widget> {
    child: WidgetPod<W>,
    width: f64,
}

impl<W: Widget> InputBox<W> {
    pub fn new(child: NewWidget<W>, width: f64) -> NewWidget<Self> {
        NewWidget::new(InputBox { child: child.to_pod(), width })
    }
}

impl<W: Widget> Widget for InputBox<W> {
    type Action = Committed;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if let Update::ChildFocusChanged(focused) = event {
            ctx.request_paint_only();
            if !focused {
                ctx.submit_action::<Committed>(Committed);
            }
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => Length::px(self.width),
            Axis::Vertical => {
                let inner = self.width - size::INPUT_PADDING_X * 2.0;
                let context = LayoutSize::maybe(Axis::Horizontal, Some(Length::px(inner)));
                let height = ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(Length::px(inner)));
                Length::px(height.get().max(size::INPUT_CONTENT_HEIGHT) + size::INPUT_PADDING_Y * 2.0)
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size_: Size) {
        let inner = Size::new(size_.width - size::INPUT_PADDING_X * 2.0, size_.height - size::INPUT_PADDING_Y * 2.0);
        let child = ctx.compute_size(&mut self.child, SizeDef::new(LenDef::FitContent(Length::px(inner.width)), LenDef::MaxContent), inner.into());
        let child = Size::new(inner.width, child.height);
        ctx.run_layout(&mut self.child, child);
        let y = (size_.height - child.height) / 2.0;
        ctx.place_child(&mut self.child, Point::new(size::INPUT_PADDING_X, y));
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::RADIUS_XL);
        painter.fill(shape, t.bg_surface).draw();
        if ctx.has_focus_target() {
            let outer = RoundedRect::from_rect(ctx.content_box().inflate(2.5, 2.5), size::RADIUS_XL + 2.5);
            painter.stroke(&outer, &Stroke::new(3.0), t.border_focus_ring).draw();
            ring(painter, shape, t.border_focus);
        } else {
            ring(painter, shape, t.border_default);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
