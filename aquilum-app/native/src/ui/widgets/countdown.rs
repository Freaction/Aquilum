use std::time::{Duration, Instant};

use masonry::accesskit::{Node, Role};
use masonry::core::{
    AccessCtx, ChildrenIds, LayoutCtx, MeasureCtx, NewWidget, PaintCtx, PropertiesMut, PropertiesRef, RegisterCtx, Update, UpdateCtx, Widget, WidgetPod,
};
use masonry::imaging::Painter;
use masonry::kurbo::{Axis, Point, Size};
use masonry::layout::{LenReq, Length};

use super::{Label, Leading, Paint};

pub struct Countdown {
    label: WidgetPod<Label>,
    at: Instant,
    text: fn(u64) -> String,
}

fn seconds_left(at: Instant, now: Instant) -> u64 {
    let left = at.saturating_duration_since(now);
    left.as_secs() + u64::from(left.subsec_nanos() > 0)
}

impl Countdown {
    pub fn new(at: Instant, text: fn(u64) -> String, paint: Paint, leading: Leading) -> NewWidget<Self> {
        let label = crate::ui::text::label(&text(seconds_left(at, Instant::now())), paint, None, leading);
        NewWidget::new(Countdown { label: label.to_pod(), at, text })
    }
}

impl Widget for Countdown {
    type Action = masonry::core::NoAction;

    fn register_children(&mut self, ctx: &mut RegisterCtx<'_>) {
        ctx.register_child(&mut self.label);
    }

    fn update(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, event: &Update) {
        if matches!(event, Update::WidgetAdded) {
            ctx.request_anim_frame();
        }
    }

    fn on_anim_frame(&mut self, ctx: &mut UpdateCtx<'_>, _props: &mut PropertiesMut<'_>, _interval: u64) {
        let left = seconds_left(self.at, Instant::now());
        let text = (self.text)(left);
        ctx.mutate_child_later(&mut self.label, move |mut label| Label::set_text(&mut label, &text));
        if left > 0 {
            crate::ui::timer::wake(ctx.widget_id(), self.at - Duration::from_secs(left - 1));
        }
    }

    fn measure(&mut self, ctx: &mut MeasureCtx<'_>, _props: &PropertiesRef<'_>, axis: Axis, len_req: LenReq, cross: Option<Length>) -> Length {
        let context = ctx.context_size();
        ctx.compute_length(&mut self.label, len_req.into(), context, axis, cross)
    }

    fn layout(&mut self, ctx: &mut LayoutCtx<'_>, _props: &PropertiesRef<'_>, size: Size) {
        ctx.run_layout(&mut self.label, size);
        ctx.place_child(&mut self.label, Point::ORIGIN);
    }

    fn paint(&mut self, _ctx: &mut PaintCtx<'_>, _props: &PropertiesRef<'_>, _painter: &mut Painter<'_>) {}

    fn accessibility_role(&self) -> Role {
        Role::GenericContainer
    }

    fn accessibility(&mut self, _ctx: &mut AccessCtx<'_>, _props: &PropertiesRef<'_>, _node: &mut Node) {}

    fn children_ids(&self) -> ChildrenIds {
        ChildrenIds::from_slice(&[self.label.id()])
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn seconds_round_up_and_stop_at_zero() {
        let now = Instant::now();
        assert_eq!(seconds_left(now + Duration::from_millis(4_300), now), 5);
        assert_eq!(seconds_left(now + Duration::from_secs(300), now), 300);
        assert_eq!(seconds_left(now, now + Duration::from_secs(1)), 0);
    }
}
