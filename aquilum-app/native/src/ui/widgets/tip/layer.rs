use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, PropertySet, RegisterCtx, Update,
    UpdateCtx, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Rect, RoundedRect, Size};
use masonry::layout::{LenReq, Length, SizeDef};
use masonry::properties::types::CrossAxisAlignment;
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

use crate::ui::theme;
use crate::ui::tokens::size;
use crate::ui::widgets::{FAST, Intro, Label, Scale, beside, kbd};

pub struct TipLayer {
    content: WidgetPod<Flex>,
    trigger: Rect,
    axis: Axis,
    pub(super) bubble: Rect,
    intro: Intro,
}

impl TipLayer {
    pub fn new(text: &str, keys: Option<&str>, trigger: Rect, axis: Axis) -> NewWidget<Self> {
        let mut row = Flex::row().cross_axis_alignment(CrossAxisAlignment::Center).with_fixed(NewWidget::new(Label::new(text, |t| t.tooltip_text).scale(Scale::Small).nowrap()));
        if let Some(keys) = keys {
            row = row.with_fixed(kbd(keys, size::SPACE_2, |t| t.tooltip_hint));
        }
        let (x, y) = (Length::px(size::TOOLTIP_PADDING_X), Length::px(size::TOOLTIP_PADDING_Y));
        let props = PropertySet::new().with(Gap::new(Length::px(size::SPACE_8))).with(Padding { top: y, bottom: y, left: x, right: x });
        let content = NewWidget::new(row).with_props(props).to_pod();
        NewWidget::new(TipLayer { content, trigger, axis, bubble: Rect::ZERO, intro: Intro::new(FAST) })
    }
}

impl Widget for TipLayer {
    type Action = masonry::core::NoAction;

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            ctx.set_opacity(self.intro.value() as f32);
            ctx.request_anim_frame();
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.intro.advance(interval) {
            ctx.request_anim_frame();
        }
        ctx.set_opacity(self.intro.value() as f32);
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.content);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, window: Size) {
        let room = Size::new(size::TOOLTIP_MAX_WIDTH.min(window.width - size::SPACE_8 * 2.0).max(0.0), window.height);
        let natural = ctx.compute_size(&mut self.content, SizeDef::MAX, room.into());
        let wanted = Size::new(natural.width.min(room.width), natural.height);
        ctx.run_layout(&mut self.content, wanted);
        let (origin, _) = beside(self.trigger, wanted, window.to_rect(), self.axis, true);
        ctx.place_child(&mut self.content, origin);
        self.bubble = Rect::from_origin_size(origin, wanted);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        painter.fill(RoundedRect::from_rect(self.bubble, size::TOOLTIP_RADIUS), theme::current().tooltip_bg).draw();
    }

    fn accepts_pointer_interaction(&self) -> bool {
        false
    }

    fn propagates_pointer_interaction(&self) -> bool {
        false
    }

    fn accessibility_role(&self) -> Role {
        Role::Tooltip
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.content.id()])
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for TipLayer {
    fn capture_pointer_event(&mut self, _ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, _event: &PointerEvent) {}
}
