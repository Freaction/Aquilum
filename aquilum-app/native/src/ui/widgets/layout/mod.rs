mod beside;
mod max_width;
mod rule;
mod slot;
mod wrap;

pub use beside::beside;
pub use max_width::*;
pub use rule::*;
pub use slot::*;
pub use wrap::*;

use masonry::core::{LayoutCtx, MeasureCtx, NewWidget, PropertySet, Widget, WidgetPod};
use masonry::kurbo::{Axis, Insets, Rect, Size};
use masonry::layout::{LayoutSize, LenDef, LenReq, Length, SizeDef};
use masonry::properties::{Gap, Padding};
use masonry::widgets::Flex;

pub fn place(ctx: &mut LayoutCtx<'_>, pod: &mut WidgetPod<dyn Widget>, rect: Rect) {
    ctx.run_layout(pod, Size::new(rect.width().max(0.0), rect.height().max(0.0)));
    ctx.place_child(pod, rect.origin());
}

pub fn inside(size: Size, insets: Insets) -> Rect {
    Rect::new(insets.x0, insets.y0, (size.width - insets.x1).max(insets.x0), (size.height - insets.y1).max(insets.y0))
}

pub fn fixed_height(axis: Axis, len_req: LenReq, height: f64, width: impl FnOnce() -> f64) -> Length {
    match (axis, len_req) {
        (Axis::Vertical, _) => Length::px(height),
        (Axis::Horizontal, LenReq::FitContent(space)) => space,
        (Axis::Horizontal, _) => Length::px(width()),
    }
}

pub fn fit_height(ctx: &mut LayoutCtx<'_>, pod: &mut WidgetPod<impl Widget + ?Sized>, width: f64, room: LayoutSize) -> f64 {
    ctx.compute_size(pod, SizeDef::new(LenDef::FitContent(Length::px(width)), LenDef::MaxContent), room).height
}

pub fn spaced(flex: Flex, gap: f64, padding: f64) -> NewWidget<Flex> {
    NewWidget::new(flex).with_props(PropertySet::new().with(Gap::new(Length::px(gap))).with(Padding::all(Length::px(padding))))
}

pub trait MaxContent {
    fn max_content(&mut self, pod: &mut WidgetPod<impl Widget + ?Sized>, axis: Axis, cross: Option<Length>) -> Length;
}

macro_rules! max_content {
    ($($ctx:ty),+) => {$(
        impl MaxContent for $ctx {
            fn max_content(&mut self, pod: &mut WidgetPod<impl Widget + ?Sized>, axis: Axis, cross: Option<Length>) -> Length {
                self.compute_length(pod, LenReq::MaxContent.into(), LayoutSize::maybe(axis.cross(), cross), axis, cross)
            }
        }
    )+};
}

max_content!(MeasureCtx<'_>, LayoutCtx<'_>);
