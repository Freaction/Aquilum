use masonry::accesskit::{Node, Role};
use masonry::core::{AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesRef, RegisterCtx, Widget, WidgetPod};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::peniko::Color;

use crate::ui::theme;
use crate::ui::tokens::size;

pub struct Rule {
    axis: Axis,
    color: fn() -> Color,
    base: Option<fn() -> Color>,
}

impl Rule {
    pub fn new(axis: Axis, color: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Rule { axis, color, base: None })
    }

    pub fn on(axis: Axis, color: fn() -> Color, base: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Rule { axis, color, base: Some(base) })
    }

    pub fn divider() -> NewWidget<Self> {
        Self::new(Axis::Horizontal, || theme::current().border_default)
    }
}

impl Widget for Rule {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, _cross: Option<Length>) -> Length {
        match (axis == self.axis, len_req) {
            (true, LenReq::FitContent(space)) => space,
            (true, _) => Length::ZERO,
            (false, _) => Length::px(size::BORDER_1),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        if let Some(base) = self.base {
            painter.fill(ctx.content_box(), base()).draw();
        }
        painter.fill(ctx.content_box(), (self.color)()).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}

pub struct Underlined {
    child: WidgetPod<dyn Widget>,
    base: fn() -> Color,
    line: fn() -> Color,
}

impl Underlined {
    pub fn new(child: NewWidget<impl Widget + ?Sized>, base: fn() -> Color, line: fn() -> Color) -> NewWidget<Self> {
        NewWidget::new(Underlined { child: child.erased().to_pod(), base, line })
    }
}

impl Widget for Underlined {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross);
        let inner = ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        if axis == Axis::Vertical { Length::px(inner.get() + size::BORDER_1) } else { inner }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let inner = Size::new(size.width, (size.height - size::BORDER_1).max(0.0));
        ctx.run_layout(&mut self.child, inner);
        ctx.place_child(&mut self.child, Point::ZERO);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        painter.fill(bounds, (self.base)()).draw();
        painter.fill(Rect::new(bounds.x0, bounds.y1 - size::BORDER_1, bounds.x1, bounds.y1), (self.line)()).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
