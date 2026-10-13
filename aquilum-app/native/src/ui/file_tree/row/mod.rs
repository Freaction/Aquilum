mod paint;
mod pointer;

pub(super) use paint::{Look, Texts, paint as paint_row};

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, AccessEvent, ChildrenIds, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent,
    PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetMut,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

use super::model::{Modifiers, Row};
use crate::ui::tokens::size;

#[derive(Debug)]
pub enum RowAction {
    Press(Modifiers),
    DragStart(Point),
    DragMove(Point),
    Drop(Point),
    ContextMenu(Point),
}

pub struct TreeRow {
    row: Row,
    pressed: bool,
    down_at: Option<Point>,
    dragging: bool,
    drop_over: bool,
    texts: Texts,
}

impl TreeRow {
    pub fn new(row: Row) -> Self {
        TreeRow { row, pressed: false, down_at: None, dragging: false, drop_over: false, texts: Texts::default() }
    }

    pub fn set_drop_over(this: &mut WidgetMut<'_, Self>, drop_over: bool) {
        if this.widget.drop_over != drop_over {
            this.widget.drop_over = drop_over;
            this.ctx.request_paint_only();
        }
    }

    fn height() -> f64 {
        size::SIDEBAR_ITEM_PADDING * 2.0 + size::SIDEBAR_ICON_SIZE
    }

}

impl Widget for TreeRow {
    type Action = RowAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        self.pointer(ctx, event);
    }

    fn on_access_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &AccessEvent) {
        if event.action == masonry::accesskit::Action::Click {
            ctx.submit_action::<RowAction>(RowAction::Press(Modifiers::default()));
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::HoveredChanged(_) => ctx.request_paint_only(),
            Update::FontsChanged => {
                self.texts.clear();
                ctx.request_layout();
            }
            _ => {}
        }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(
        &mut self,
        _ctx: &mut MeasureCtx<'_>,
        _props: &PropertiesRef<'_>,
        axis: Axis,
        len_req: LenReq,
        _cross_length: Option<Length>,
    ) -> Length {
        match (axis, len_req) {
            (Axis::Vertical, _) => Length::px(Self::height()),
            (Axis::Horizontal, LenReq::FitContent(space)) => space,
            (Axis::Horizontal, _) => Length::px(paint::text_x(self.row.depth) + size::SIDEBAR_ITEM_PADDING),
        }
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, _size: Size) {}

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let look = Look { hovered: ctx.is_hovered(), drop_over: self.drop_over };
        let bounds = ctx.content_box();
        paint_row(ctx, painter, bounds, &self.row, look, &mut self.texts);
    }

    fn get_cursor(&self, _ctx: &masonry::core::QueryCtx<'_>, _pos: Point) -> masonry::core::CursorIcon {
        if self.dragging { masonry::core::CursorIcon::Grabbing } else { masonry::core::CursorIcon::Default }
    }

    fn accessibility_role(&self) -> Role {
        Role::TreeItem
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.row.name.clone());
        if self.row.folder {
            node.set_expanded(self.row.expanded);
        }
        node.set_selected(self.row.selected || self.row.active);
        node.add_action(masonry::accesskit::Action::Click);
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
