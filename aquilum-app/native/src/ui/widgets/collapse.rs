use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, UpdateCtx, Widget, WidgetMut,
    WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Rect, Size};
use masonry::layout::{LenReq, Length};
use masonry::peniko::Color;

use crate::ui::motion::{self, EASE_IN, EASE_IN_OUT, approach};

pub fn bezier((x1, y1, x2, y2): (f64, f64, f64, f64), t: f64) -> f64 {
    if t <= 0.0 || t >= 1.0 {
        return t.clamp(0.0, 1.0);
    }
    let curve = |t: f64, a: f64, b: f64| 3.0 * a * t * (1.0 - t) * (1.0 - t) + 3.0 * b * t * t * (1.0 - t) + t * t * t;
    let (mut lo, mut hi) = (0.0, 1.0);
    for _ in 0..24 {
        let mid = (lo + hi) / 2.0;
        if curve(mid, x1, x2) < t { lo = mid } else { hi = mid }
    }
    curve((lo + hi) / 2.0, y1, y2)
}

pub struct Collapse {
    child: WidgetPod<dyn Widget>,
    width: f64,
    open: bool,
    progress: f64,
    fade: f64,
    background: fn() -> Color,
}

impl Collapse {
    pub fn new(child: NewWidget<dyn Widget>, width: f64, open: bool, from: bool, background: fn() -> Color) -> NewWidget<Self> {
        let start = if from { 1.0 } else { 0.0 };
        NewWidget::new(Collapse { child: child.to_pod(), width, open, progress: start, fade: start, background })
    }

    pub fn set_open(this: &mut WidgetMut<'_, Self>, open: bool) {
        if this.widget.open != open {
            this.widget.open = open;
            this.ctx.set_stashed(&mut this.widget.child, false);
            this.ctx.request_anim_frame();
        }
    }

    fn goal(&self) -> f64 {
        if self.open { 1.0 } else { 0.0 }
    }

    fn shown(&self) -> f64 {
        self.width * bezier(EASE_IN_OUT, self.progress)
    }
}

impl Widget for Collapse {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        let seconds = interval as f64 / 1e9;
        let goal = self.goal();
        self.progress = approach(self.progress, goal, motion::step(seconds, motion::NORMAL));
        self.fade = approach(self.fade, goal, motion::step(seconds, motion::FAST));
        ctx.request_layout();
        if self.progress != goal || self.fade != goal {
            ctx.request_anim_frame();
        } else if !self.open {
            ctx.set_stashed(&mut self.child, true);
        }
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &masonry::core::Update) {
        if matches!(event, masonry::core::Update::WidgetAdded) {
            if self.progress != self.goal() {
                ctx.request_anim_frame();
            } else if !self.open {
                ctx.set_stashed(&mut self.child, true);
            }
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, _len_req: LenReq, _cross: Option<Length>) -> Length {
        match axis {
            Axis::Horizontal => Length::px(self.shown()),
            Axis::Vertical => ctx.context_size().length(axis).unwrap_or(Length::ZERO),
        }
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        if !ctx.child_is_stashed(&self.child) {
            ctx.run_layout(&mut self.child, Size::new(self.width, size.height));
            ctx.place_child(&mut self.child, Point::ORIGIN);
        }
        ctx.set_clip_path(Rect::from_origin_size(Point::ORIGIN, size));
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn post_paint(&mut self, ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, painter: &mut Painter<'_>) {
        let opacity = if self.open { bezier(EASE_IN, self.fade) } else { 1.0 - bezier(EASE_IN, 1.0 - self.fade) };
        let veil = 1.0 - opacity;
        if veil > 0.0 {
            painter.fill(ctx.content_box(), (self.background)().multiply_alpha(veil as f32)).draw();
        }
    }

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.child.id()])
    }
}
