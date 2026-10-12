use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetPod};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};

pub struct MaxWidth<W: Widget> {
    child: WidgetPod<W>,
    max: f64,
}

impl<W: Widget> MaxWidth<W> {
    pub fn new(child: NewWidget<W>, max: f64) -> NewWidget<Self> {
        NewWidget::new(MaxWidth { child: child.to_pod(), max })
    }
}

impl<W: Widget> Widget for MaxWidth<W> {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => match len_req {
                LenReq::MinContent => Length::ZERO,
                LenReq::MaxContent => Length::px(self.max),
                LenReq::FitContent(space) => space,
            },
            Axis::Vertical => {
                let inner = Length::px(cross.map_or(self.max, |c| c.get().min(self.max)));
                let context = LayoutSize::maybe(Axis::Horizontal, Some(inner));
                ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), context, Axis::Vertical, Some(inner))
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = size.width.min(self.max);
        let auto = SizeDef::new(LenDef::FitContent(Length::px(inner)), LenDef::MaxContent);
        let child = ctx.compute_size(&mut self.child, auto, Size::new(inner, size.height).into());
        let child = Size::new(inner, child.height);
        ctx.run_layout(&mut self.child, child);
        ctx.place_child(&mut self.child, Point::new(((size.width - inner) / 2.0).round(), 0.0));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
