use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, CursorIcon, EventCtx, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, QueryCtx, RegisterCtx,
    TextEvent, Update, UpdateCtx, Widget, render_text,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Point, Size};
use masonry::layout::{LenReq, Length};
use masonry::parley::Selection;

use super::{Label, select};
use crate::ui::theme;

impl Widget for Label {
    type Action = masonry::core::NoAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::FontsChanged => {
                self.cached = None;
                ctx.request_layout();
            }
            Update::FocusChanged(false) if self.selectable => {
                self.selection = Selection::default();
                ctx.request_paint_only();
            }
            _ => {}
        }
    }

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if self.selectable {
            self.pointer(ctx, event);
        }
    }

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        if self.selectable {
            self.keyboard(ctx, event);
        }
    }

    fn get_cursor(&self, _ctx: &QueryCtx<'_>, _pos: Point) -> CursorIcon {
        if self.selectable { CursorIcon::Text } else { CursorIcon::Default }
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let (fcx, lcx) = ctx.text_contexts();
        match axis {
            Axis::Horizontal => {
                let max = f64::from(self.layout(fcx, lcx, None).width()).ceil();
                let min = if self.wrap { f64::from(self.layout(fcx, lcx, Some(0.0)).width()).ceil() } else { 0.0 };
                match len_req {
                    LenReq::MaxContent => Length::px(max),
                    LenReq::MinContent => Length::px(min),
                    LenReq::FitContent(space) => Length::px(max.min(space.get().max(min))),
                }
            }
            Axis::Vertical => {
                let width = cross.map(|w| w.get() as f32);
                Length::px(f64::from(self.layout(fcx, lcx, width).height()).ceil())
            }
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        if !self.wrap {
            ctx.set_clip_path(size.to_rect());
        }
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let bounds = ctx.content_box();
        let tm = theme::current();
        if let Some(background) = self.background {
            painter.fill(ctx.border_box(), background(&tm)).draw();
        }
        let color = (self.paint)(&tm);
        let (fcx, lcx) = ctx.text_contexts();
        let selection = self.selection;
        let layout = self.layout(fcx, lcx, Some(bounds.width() as f32));
        select::paint_selection(painter, layout, selection, bounds.origin().to_vec2());
        render_text(painter, Affine::translate(bounds.origin().to_vec2()), layout, &[color.into()], true);
    }

    fn accessibility_role(&self) -> Role {
        Role::Label
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_value(self.text.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }
}
