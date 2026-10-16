use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PointerButton, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx,
    RegisterCtx, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

pub const SASH_WIDTH: f64 = 4.0;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Edge {
    Left,
    Right,
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SashAction {
    Press,
    Drag(f64),
    Release,
    Reset,
}

#[derive(Default)]
pub struct Sash {
    from: Option<f64>,
}

impl Sash {
    pub fn new() -> NewWidget<Self> {
        NewWidget::new(Sash::default())
    }
}

pub struct Edged {
    content: WidgetPod<dyn Widget>,
    sash: WidgetPod<Sash>,
    edge: Edge,
}

impl Edged {
    pub fn new(content: NewWidget<dyn Widget>, sash: NewWidget<Sash>, edge: Edge) -> NewWidget<Self> {
        NewWidget::new(Edged { content: content.to_pod(), sash: sash.to_pod(), edge })
    }
}

impl Widget for Edged {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.content);
        ctx.register_child(&mut self.sash);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.content, size);
        ctx.place_child(&mut self.content, Point::ORIGIN);
        ctx.run_layout(&mut self.sash, Size::new(SASH_WIDTH, size.height));
        let x = if self.edge == Edge::Right { size.width - SASH_WIDTH } else { 0.0 };
        ctx.place_child(&mut self.sash, Point::new(x, 0.0));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.content.id(), self.sash.id()])
    }
}

impl Widget for Sash {
    type Action = SashAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let x = |position: masonry::dpi::PhysicalPosition<f64>, ctx: &EventCtx<'_>| position.x / ctx.scale_factor();
        match event {
            PointerEvent::Down(e) if e.button == Some(PointerButton::Primary) => {
                if e.state.count >= 2 {
                    ctx.submit_action::<SashAction>(SashAction::Reset);
                } else {
                    self.from = Some(x(e.state.position, ctx));
                    ctx.capture_pointer();
                    ctx.submit_action::<SashAction>(SashAction::Press);
                }
                ctx.set_handled();
            }
            PointerEvent::Move(update) => {
                if let Some(from) = self.from {
                    ctx.submit_action::<SashAction>(SashAction::Drag(x(update.current.position, ctx) - from));
                    ctx.set_handled();
                }
            }
            PointerEvent::Up(_) | PointerEvent::Cancel(_) => {
                if self.from.take().is_some() {
                    ctx.submit_action::<SashAction>(SashAction::Release);
                }
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => Length::px(SASH_WIDTH),
            Axis::Vertical => ctx.context_size().length(axis).unwrap_or(Length::ZERO),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::EwResize
    }

    fn accessibility_role(&self) -> Role {
        Role::Splitter
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
