use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, CursorIcon, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx,
    PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx, Widget, WidgetId,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, RoundedRect, Size};
use masonry::layout::{LenReq, Length};

use crate::ui::keymap::gesture::TOUCH_REACH;
use crate::ui::theme;
use crate::ui::tokens::size::SELECTION_HANDLE_SIZE;

pub struct Grip<const END: bool> {
    editor: WidgetId,
}

impl<const END: bool> Grip<END> {
    pub fn new(editor: WidgetId) -> NewWidget<Self> {
        NewWidget::new(Grip { editor })
    }

    pub fn frame(tip: Point) -> Rect {
        let x = if END { tip.x } else { tip.x - SELECTION_HANDLE_SIZE };
        Rect::new(x, tip.y, x + SELECTION_HANDLE_SIZE, tip.y + SELECTION_HANDLE_SIZE).inflate(TOUCH_REACH, TOUCH_REACH)
    }
}

impl<const END: bool> Widget for Grip<END> {
    type Action = NoAction;

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, _axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(SELECTION_HANDLE_SIZE + TOUCH_REACH * 2.0)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let round = SELECTION_HANDLE_SIZE / 2.0;
        let radii = if END { (0.0, round, round, round) } else { (round, 0.0, round, round) };
        let shape = ctx.content_box().inflate(-TOUCH_REACH, -TOUCH_REACH);
        painter.fill(RoundedRect::from_rect(shape, radii), theme::current().caret_color).draw();
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        CursorIcon::Grab
    }

    fn pointer_delegate(&self) -> Option<WidgetId> {
        Some(self.editor)
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl<const END: bool> Layer for Grip<END> {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
