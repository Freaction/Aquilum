use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx,
    TextEvent, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, RoundedRect, Size, Stroke};
use masonry::layout::{LayoutSize, LenReq, Length};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use super::item::MenuItem;
use super::{Entry, MenuChoice, box_shadow};
use crate::ui::scroll::ScrollArea;
use crate::ui::theme;
use crate::ui::tokens::size;

pub struct Menu {
    child: WidgetPod<ScrollArea<Flex>>,
    label: String,
    width: Option<f64>,
    max_width: f64,
    max_height: f64,
    exclude: Option<Rect>,
}

impl Menu {
    pub const LIST_MAX_HEIGHT: f64 = 256.0;

    const MIN_WIDTH: f64 = 128.0;

    pub fn new(label: &str, items: &[&str]) -> NewWidget<Self> {
        Self::build(label, &plain_entries(items), None, None)
    }

    pub fn entries(label: &str, entries: &[Entry<'_>]) -> NewWidget<Self> {
        Self::build(label, entries, None, None)
    }

    pub fn list(label: &str, items: &[&str], checked: usize, trigger: Rect) -> NewWidget<Self> {
        let mut menu = Self::build(label, &plain_entries(items), Some(checked), Some(trigger.width()));
        menu.widget.max_height = Self::LIST_MAX_HEIGHT;
        menu.widget.exclude = Some(trigger);
        menu
    }

    pub fn dropdown(label: &str, items: &[&str], checked: usize, trigger: Rect, window: Size, width: f64) -> NewWidget<Self> {
        let mut menu = Self::list(label, items, checked, trigger);
        menu.widget.width = Some(width);
        menu.widget.max_width = width;
        menu.widget.max_height = (window.height - Self::dropdown_origin(trigger, window, width).y - size::SPACE_8).max(MenuItem::height());
        menu
    }

    pub fn context_origin(at: Point, items: usize, window: Size) -> Point {
        let size = Self::estimated_size(items);
        let x = if at.x + size.width > window.width { (at.x - size.width).max(size::SPACE_8) } else { at.x };
        let y = if at.y + size.height > window.height { (at.y - size.height).max(size::SPACE_8) } else { at.y };
        Point::new(x, y)
    }

    pub fn list_origin(trigger: Rect, items: usize, window: Size) -> Point {
        let height = Self::estimated_size(items).height.min(Self::LIST_MAX_HEIGHT);
        let below = trigger.y1 + size::SPACE_4;
        let y = if below + height > window.height - size::SPACE_8 { (trigger.y0 - size::SPACE_4 - height).max(size::SPACE_8) } else { below };
        Point::new(trigger.x0, y)
    }

    pub fn dropdown_origin(trigger: Rect, window: Size, width: f64) -> Point {
        let right = (window.width - width - size::SPACE_8).max(size::SPACE_8);
        Point::new((trigger.x1 - width).clamp(size::SPACE_8, right), trigger.y1 + size::SPACE_4)
    }

    fn build(label: &str, items: &[Entry<'_>], checked: Option<usize>, width: Option<f64>) -> NewWidget<Self> {
        let mut column = Flex::column().cross_axis_alignment(CrossAxisAlignment::Stretch);
        for (index, item) in items.iter().enumerate() {
            column = column.with_fixed(NewWidget::new(MenuItem::new(index, item.label, item.icon, item.disabled, checked == Some(index), checked.is_some())));
        }
        let column = NewWidget::new(column).with_props(
            PropertySet::new()
                .with(Gap::new(Length::px(size::GAP_XS)))
                .with(Padding::all(Length::px(size::GAP_XS))),
        );
        let scroll = NewWidget::new(ScrollArea::new(column));
        NewWidget::new(Menu { child: scroll.to_pod(), label: label.to_owned(), width, max_width: f64::INFINITY, max_height: f64::INFINITY, exclude: None })
    }

    pub fn estimated_size(items: usize) -> Size {
        let items = items as f64;
        let height = items * MenuItem::height() + (items + 1.0) * size::GAP_XS;
        Size::new(Self::MIN_WIDTH + 32.0, height)
    }
}

fn plain_entries<'a>(items: &[&'a str]) -> Vec<Entry<'a>> {
    items.iter().map(|&label| Entry { label, icon: None, disabled: false }).collect()
}

impl Widget for Menu {
    type Action = MenuChoice;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        let close = match event {
            TextEvent::Keyboard(k) => k.state == KeyState::Down && k.key == Key::Named(NamedKey::Escape),
            TextEvent::WindowFocusChange(false) => true,
            _ => false,
        };
        if close {
            ctx.submit_action::<MenuChoice>(MenuChoice::Dismissed);
            ctx.remove_layer(ctx.widget_id());
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = LayoutSize::maybe(axis.cross(), cross);
        let length = ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross);
        match (axis, self.width) {
            (Axis::Horizontal, Some(width)) => Length::px(width.max(length.get()).max(Self::MIN_WIDTH).min(self.max_width)),
            (Axis::Horizontal, None) => Length::px(length.get().max(Self::MIN_WIDTH).min(self.max_width)),
            (Axis::Vertical, _) => Length::px(length.get().min(self.max_height)),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let t = theme::current();
        let bounds = ctx.content_box();
        let shape = RoundedRect::from_rect(bounds, size::RADIUS_XL);
        box_shadow(painter, shape, t.elevation_float);
        painter.fill(shape, t.dialog_bg).draw();
        for ring in t.shadow_ring {
            painter.stroke(&shape, &Stroke::new(ring.spread), ring.color).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::Menu
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_label(self.label.clone());
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
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
