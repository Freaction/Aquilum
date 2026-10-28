use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::KeyState;
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Update,
    UpdateCtx, Widget, WidgetMut, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::widgets::Flex;

use super::sub::Sub;
use super::{MenuChoice, box_shadow};
use crate::ui::scroll::ScrollArea;
use crate::ui::theme;
use crate::ui::tokens::size;

pub struct Menu {
    pub(super) child: WidgetPod<ScrollArea<Flex>>,
    pub(super) label: String,
    pub(super) width: Option<f64>,
    pub(super) max_width: f64,
    pub(super) max_height: f64,
    pub(super) exclude: Option<Rect>,
    pub(super) sub: Option<Sub>,
    pub(super) main: Size,
}

impl Menu {
    pub(super) const MIN_WIDTH: f64 = 128.0;

    pub fn show_sub(this: &mut WidgetMut<'_, Self>, shown: bool) {
        let Some(sub) = &mut this.widget.sub else { return };
        if sub.shown != shown {
            sub.shown = shown;
            this.ctx.set_stashed(&mut sub.panel, !shown);
            this.ctx.request_layout();
        }
    }
}

fn paint_panel(painter: &mut Painter<'_>, rect: Rect) {
    let t = theme::current();
    let shape = RoundedRect::from_rect(rect, size::RADIUS_XL);
    box_shadow(painter, shape, t.elevation_float);
    painter.fill(shape, t.dialog_bg).draw();
    for ring in t.shadow_ring {
        painter.stroke(&shape, &Stroke::new(ring.spread), ring.color).draw();
    }
}

impl Widget for Menu {
    type Action = MenuChoice;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let close = match event {
            TextEvent::Keyboard(k) => k.state == KeyState::Down && crate::ui::keymap::command(&[crate::ui::keymap::Scope::Dialog], &k.key, k.modifiers) == Some(crate::ui::keymap::Command::Close),
            TextEvent::WindowFocusChange(false) => true,
            _ => false,
        };
        if close {
            ctx.submit_action::<MenuChoice>(MenuChoice::Dismissed);
            ctx.remove_layer(ctx.widget_id());
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded)
            && let Some(sub) = &mut self.sub
        {
            ctx.set_stashed(&mut sub.panel, true);
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
        if let Some(sub) = &mut self.sub {
            ctx.register_child(&mut sub.panel);
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross);
        let length = ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        let main = match (axis, self.width) {
            (Axis::Horizontal, Some(width)) => width.max(length.get()).max(Self::MIN_WIDTH).min(self.max_width),
            (Axis::Horizontal, None) => length.get().max(Self::MIN_WIDTH).min(self.max_width),
            (Axis::Vertical, _) => length.get().min(self.max_height),
        };
        match axis {
            Axis::Horizontal => self.main.width = main,
            Axis::Vertical => self.main.height = main,
        }
        let Some(sub) = self.sub.as_mut().filter(|sub| sub.shown) else { return Length::px(main) };
        let panel = sub.measure(ctx);
        match axis {
            Axis::Horizontal => Length::px(main + size::GAP_XS + panel.width),
            Axis::Vertical => Length::px(main.max(sub.top + panel.height)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let shown = self.sub.as_ref().is_some_and(|sub| sub.shown);
        let main = if shown { Size::new(self.main.width, size.height.min(self.main.height)) } else { size };
        self.main = main;
        ctx.run_layout(&mut self.child, main);
        ctx.place_child(&mut self.child, Point::ORIGIN);
        if let Some(sub) = self.sub.as_mut().filter(|sub| sub.shown) {
            let origin = Point::new(main.width + size::GAP_XS, sub.top);
            sub.place(ctx, origin);
        }
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        paint_panel(painter, Rect::from_origin_size(Point::ORIGIN, self.main));
        if let Some(sub) = self.sub.as_ref().filter(|sub| sub.shown) {
            paint_panel(painter, sub.rect);
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Menu
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        match &self.sub {
            Some(sub) => ChildrenIds::from_slice(&[self.child.id(), sub.panel.id()]),
            None => ChildrenIds::from_slice(&[self.child.id()]),
        }
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for Menu {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        let (position, scroll) = match event {
            PointerEvent::Down(e) => (e.state.position, false),
            PointerEvent::Scroll(e) => (e.state.position, true),
            _ => return,
        };
        let local = ctx.local_position(position);
        let window = ctx.to_window(local);
        if ctx.border_box().contains(local) || !scroll && self.exclude.is_some_and(|r| r.contains(window)) {
            return;
        }
        ctx.submit_action::<MenuChoice>(MenuChoice::Dismissed);
        ctx.remove_layer(ctx.widget_id());
    }
}
