use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetPod};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

use super::{MaxContent, fit_height};

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
            Axis::Vertical => ctx.max_content(&mut self.child, Axis::Vertical, Some(Length::px(cross.map_or(self.max, |c| c.get().min(self.max))))),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = size.width.min(self.max);
        let child = Size::new(inner, fit_height(ctx, &mut self.child, inner, Size::new(inner, size.height).into()));
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
