use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

use super::collapse::bezier;

pub const EASE_OUT: (f64, f64, f64, f64) = (0.0, 0.0, 0.2, 1.0);
pub const EASE_STANDARD: (f64, f64, f64, f64) = (0.4, 0.0, 0.2, 1.0);
pub const LINEAR: (f64, f64, f64, f64) = (0.0, 0.0, 1.0, 1.0);
pub const NORMAL: f64 = 0.2;
pub const FAST: f64 = 0.1;

pub struct Intro {
    seconds: f64,
    elapsed: f64,
}

impl Intro {
    pub fn new(seconds: f64) -> Self {
        Intro { seconds, elapsed: 0.0 }
    }

    pub fn restart(&mut self) {
        self.elapsed = 0.0;
    }

    pub fn reverse(&mut self) {
        self.elapsed = (self.seconds - self.elapsed).max(0.0);
    }

    pub fn finish(&mut self) {
        self.elapsed = self.seconds;
    }

    pub fn value(&self) -> f64 {
        bezier(EASE_OUT, (self.elapsed / self.seconds).min(1.0))
    }

    pub fn reversed(&self) -> f64 {
        bezier(EASE_OUT, 1.0 - (self.elapsed / self.seconds).min(1.0))
    }

    pub fn running(&self) -> bool {
        self.elapsed < self.seconds
    }

    pub fn advance(&mut self, interval: u64) -> bool {
        self.elapsed += interval as f64 / 1e9;
        self.elapsed < self.seconds
    }
}

pub struct Appear {
    child: WidgetPod<dyn Widget>,
    intro: Intro,
}

impl Appear {
    pub fn new(child: NewWidget<impl Widget + ?Sized>, seconds: f64) -> NewWidget<Self> {
        NewWidget::new(Appear { child: child.erased().to_pod(), intro: Intro::new(seconds) })
    }

    fn apply(&self, ctx: &mut UpdateCtx<'_>) {
        ctx.set_opacity(self.intro.value() as f32);
    }
}

impl Widget for Appear {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.child);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        match event {
            Update::WidgetAdded | Update::StashedChanged(false) => {
                self.intro.restart();
                self.apply(ctx);
                ctx.request_anim_frame();
            }
            Update::StashedChanged(true) => self.intro.finish(),
            _ => {}
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, interval: u64) {
        if self.intro.advance(interval) {
            ctx.request_anim_frame();
        }
        self.apply(ctx);
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = ctx.context_size();
        ctx.compute_length(&mut self.child, len_req.into(), context, axis, cross)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.child, size);
        ctx.place_child(&mut self.child, Point::ORIGIN);
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

pub struct Tween {
    from: f64,
    to: f64,
    elapsed: f64,
    seconds: f64,
    curve: (f64, f64, f64, f64),
}

impl Tween {
    pub fn new(value: f64, seconds: f64, curve: (f64, f64, f64, f64)) -> Self {
        Tween { from: value, to: value, elapsed: seconds, seconds, curve }
    }

    pub fn value(&self) -> f64 {
        self.from + (self.to - self.from) * bezier(self.curve, (self.elapsed / self.seconds).min(1.0))
    }

    pub fn set(&mut self, to: f64) -> bool {
        if to == self.to {
            return false;
        }
        self.from = self.value();
        self.to = to;
        self.elapsed = 0.0;
        true
    }

    pub fn advance(&mut self, interval: u64) -> bool {
        self.elapsed += interval as f64 / 1e9;
        self.elapsed < self.seconds
    }

    pub fn finish(&mut self) {
        self.elapsed = self.seconds;
    }
}
