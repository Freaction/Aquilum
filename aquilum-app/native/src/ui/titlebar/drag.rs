use masonry::core::PropertySet;
use masonry::properties::Padding;
use masonry::widgets::SizedBox;

use super::*;
use crate::ui::style;
use crate::ui::widgets::BAR_HEIGHT;

pub fn panel_header(header: SizedBox, padding: Padding) -> NewWidget<DragArea> {
    let header = NewWidget::new(header.height(Length::px(BAR_HEIGHT))).with_props(PropertySet::new().with(padding));
    DragArea::new(header.with_class(style::TABS))
}

pub struct DragArea {
    child: WidgetPod<dyn Widget>,
}

impl DragArea {
    pub fn new(child: NewWidget<impl Widget + ?Sized>) -> NewWidget<Self> {
        NewWidget::new(DragArea { child: child.erased().to_pod() })
    }
}

impl Widget for DragArea {
    type Action = TitlebarAction;

    fn on_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if WINDOW_CHROME
            && ctx.pointer_capture_target_id().is_none()
            && let PointerEvent::Down(e) = event
            && pointer::press(e.button)
        {
            let action = if e.state.count >= 2 { TitlebarAction::ToggleMaximize } else { TitlebarAction::Drag };
            ctx.submit_action::<TitlebarAction>(action);
        }
    }

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        ctx.compute_length(&mut self.child, len_req.into(), LayoutSize::maybe(axis.cross(), cross), axis, cross)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ZERO);
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
