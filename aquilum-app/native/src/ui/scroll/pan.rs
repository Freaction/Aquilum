use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PointerEvent, PointerScrollEvent, PropertiesMut, PropertiesRef,
    RegisterCtx, ScrollDelta, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

#[derive(Default)]
pub struct Pan {
    pub offset: f64,
    content: f64,
    width: f64,
}

impl Pan {
    pub fn width(&self) -> f64 {
        self.width
    }

    pub fn resize(&mut self, content: f64, width: f64) -> bool {
        let changed = self.width != width;
        (self.content, self.width) = (content, width);
        self.shift(0.0);
        changed
    }

    pub fn reveal(&mut self, left: f64, width: f64) {
        self.offset = self.offset.max(left + width - self.width).min(left);
        self.shift(0.0);
    }

    pub fn shift(&mut self, d: f64) -> bool {
        let before = self.offset;
        self.offset = (self.offset + d).clamp(0.0, (self.content - self.width).max(0.0));
        self.offset != before
    }

    pub fn wheel(&mut self, ctx: &mut EventCtx<'_>, delta: &ScrollDelta) {
        let d = super::pixels(delta, ctx.scale_factor(), self.width);
        if self.shift(-if d.x != 0.0 { d.x } else { d.y }) {
            ctx.request_layout();
            ctx.set_handled();
        }
    }
}

pub struct HScroll {
    child: WidgetPod<dyn Widget>,
    pan: Pan,
}

impl HScroll {
    pub fn new(child: NewWidget<dyn Widget>) -> NewWidget<Self> {
        NewWidget::new(HScroll { child: child.to_pod(), pan: Pan::default() })
    }
}

impl Widget for HScroll {
    type Action = NoAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Scroll(PointerScrollEvent { delta, .. }) = event {
            self.pan.wheel(ctx, delta);
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        if axis == Axis::Horizontal && matches!(len_req, LenReq::MinContent) {
            return Length::ZERO;
        }
        let context = ctx.context_size();
        ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let content = ctx.compute_length(&mut self.child, LenReq::MaxContent.into(), size.into(), Axis::Horizontal, Some(Length::px(size.height))).get();
        self.pan.resize(content, size.width);
        ctx.run_layout(&mut self.child, Size::new(content.max(size.width), size.height));
        ctx.place_child(&mut self.child, Point::new(-self.pan.offset, 0.0));
        ctx.set_clip_path(size.to_rect());
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
