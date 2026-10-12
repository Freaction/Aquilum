use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetMut, WidgetPod};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

pub struct Slot {
    child: WidgetPod<dyn Widget>,
    fit: bool,
}

impl Slot {
    pub fn new(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: false })
    }

    pub fn fit(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(Slot { child: child.to_pod(), fit: true })
    }

    pub fn set_child(this: &mut WidgetMut<'_, Self>, child: NewWidget<dyn Widget>) {
        this.ctx.remove_child(std::mem::replace(&mut this.widget.child, child.to_pod()));
        this.ctx.children_changed();
    }
}

impl Widget for Slot {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        if self.fit {
            let context = ctx.context_size();
            return ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        }
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
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
