mod max_width;
mod rule;
mod slot;
mod wrap;

pub use max_width::*;
pub use rule::*;
pub use slot::*;
pub use wrap::*;

use masonry::core::{LayoutCtx, Widget, WidgetPod};
use masonry::kurbo::{Rect, Size};

pub fn place(ctx: &mut LayoutCtx<'_>, pod: &mut WidgetPod<dyn Widget>, rect: Rect) {
    ctx.run_layout(pod, Size::new(rect.width().max(0.0), rect.height().max(0.0)));
    ctx.place_child(pod, rect.origin());
}
