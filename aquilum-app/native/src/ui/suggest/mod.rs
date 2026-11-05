use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, NoAction, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, Widget,
    WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, RoundedRect, Size};
use masonry::layout::{LenReq, Length};

use super::menu::box_shadow;
use super::scroll::ScrollArea;
use super::theme;
use super::tokens::size;

mod rows;

use rows::Rows;
pub use rows::SuggestPicked;

pub struct SuggestList {
    scroll: WidgetPod<ScrollArea<Rows>>,
    size: Size,
}

impl SuggestList {
    pub fn new(items: Vec<String>, query: String, selected: usize, size: Size) -> NewWidget<Self> {
        let rows = NewWidget::new(Rows::new(items, query, selected));
        NewWidget::new(SuggestList { scroll: NewWidget::new(ScrollArea::new(rows)).to_pod(), size })
    }

    pub fn set(this: &mut WidgetMut<'_, Self>, items: Vec<String>, query: String, selected: usize, size: Size) {
        if this.widget.size != size {
            this.widget.size = size;
            this.ctx.request_layout();
        }
        let mut scroll = this.ctx.get_mut(&mut this.widget.scroll);
        Rows::set(&mut ScrollArea::child_mut(&mut scroll), items, query, selected);
        let (top, bottom) = Rows::span(selected);
        ScrollArea::reveal(&mut scroll, top, bottom);
    }

    pub fn size_for(items: &[String]) -> Size {
        let rows = Rows::size_for(items);
        Size::new(rows.width, rows.height.min(size::SUGGEST_MAX_HEIGHT))
    }
}

impl Widget for SuggestList {
    type Action = NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.scroll);
    }

    fn measure(&mut self, _ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        Length::px(self.size.get_coord(axis))
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.scroll, size);
        ctx.place_child(&mut self.scroll, Point::ORIGIN);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        let shape = RoundedRect::from_rect(ctx.content_box(), size::RADIUS_XL);
        box_shadow(painter, shape, tm.elevation_float);
        box_shadow(painter, shape, tm.shadow_ring);
        painter.fill(shape, tm.dialog_bg).draw();
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.scroll.id()])
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for SuggestList {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
