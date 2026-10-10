use masonry::accesskit::{Node, Role};
use masonry::core::keyboard::{Key, KeyState, NamedKey};
use masonry::core::{
    AccessCtx, ChildrenIds, EventCtx, Layer, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PointerEvent, PropertiesMut, PropertiesRef, RegisterCtx, TextEvent, Widget,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Affine, Axis, Rect, RoundedRect, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::ImageBrush;

use super::menu::box_shadow;
use super::theme;

const MARGIN: f64 = 32.0;
const RADIUS: f64 = 8.0;

#[derive(Debug)]
pub struct ViewerClosed;

pub struct Viewer {
    image: ImageBrush,
    frame: Rect,
}

impl Viewer {
    pub fn new(image: ImageBrush) -> NewWidget<Self> {
        NewWidget::new(Viewer { image, frame: Rect::ZERO })
    }
}

impl Widget for Viewer {
    type Action = ViewerClosed;

    fn on_text_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &TextEvent) {
        if let TextEvent::Keyboard(k) = event
            && k.state == KeyState::Down
            && k.key == Key::Named(NamedKey::Escape)
        {
            ctx.submit_action::<ViewerClosed>(ViewerClosed);
        }
    }

    fn accepts_focus(&self) -> bool {
        true
    }

    fn register_children(&mut self, _ctx: &mut RegisterCtx<'_>) {}

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        ctx.context_size().length(axis).unwrap_or(Length::ZERO)
    }

    fn layout(&mut self, _ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        let (iw, ih) = (f64::from(self.image.image.width), f64::from(self.image.image.height));
        let room = Size::new((size.width - MARGIN * 2.0).max(1.0), (size.height - MARGIN * 2.0).max(1.0));
        let k = (room.width / iw).min(room.height / ih).min(1.0);
        let (w, h) = (iw * k, ih * k);
        self.frame = Rect::new((size.width - w) / 2.0, (size.height - h) / 2.0, (size.width + w) / 2.0, (size.height + h) / 2.0);
    }

    fn paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let tm = theme::current();
        painter.fill(ctx.content_box(), tm.bg_overlay).draw();
        let shape = RoundedRect::from_rect(self.frame, RADIUS);
        box_shadow(painter, shape, tm.shadow_action);
        let k = self.frame.width() / f64::from(self.image.image.width);
        let image = &self.image;
        let origin = self.frame.origin();
        painter.with_fill_clip(shape, |painter| painter.draw_image(image, Affine::translate(origin.to_vec2()) * Affine::scale(k)));
    }

    fn accessibility_role(&self) -> Role {
        Role::Dialog
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, node: &mut Node) {
        node.set_modal();
    }

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::new()
    }

    fn as_layer(&mut self) -> Option<&mut dyn Layer> {
        Some(self)
    }
}

impl Layer for Viewer {
    fn capture_pointer_event(&mut self, ctx: &mut EventCtx<'_>, _props: &mut PropertiesMut<'_>, event: &PointerEvent) {
        if let PointerEvent::Down(_) = event {
            ctx.submit_action::<ViewerClosed>(ViewerClosed);
        }
    }
}
